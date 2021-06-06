//! Implementación de `runtime::sys` para Espressif ESP8266.
//!
//! De momento, la ausencia de atómicos funcionales en `rust-xtensa`
//! obliga a utilizar más `unsafe` de lo ideal. Como esta es una
//! plataforma `#![no_std]`, este módulo debe implementar un punto
//! de entrada específico a la plataforma y un panic handler.

use buddy_system_allocator::LockedHeap;
use core::convert::Infallible;
use xtensa_lx::mutex::{CriticalSectionMutex, Mutex};

use esp8266_hal::{
    ehal::digital::v2::OutputPin,
    gpio::{self, Output, PushPull},
    interrupt::*,
    prelude::*,
    target::{Peripherals, DPORT, TIMER},
    uart::{UART0Ext, UART0Serial},
};

use crate::{
    chrono::{Duration, Ticks},
    matrix::Display,
};

mod atomic;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

extern "C" {
    static _heap_start: u8;
    static _heap_end: u8;
}

pub static SERIAL: CriticalSectionMutex<Option<UART0Serial>> = CriticalSectionMutex::new(None);

static HW: CriticalSectionMutex<Option<Hw>> = CriticalSectionMutex::new(None);

macro_rules! sys_debug {
    ($($b:tt)*) => {
        {
            use core::fmt::Write;
            use xtensa_lx::mutex::Mutex;

            (&crate::esp8266::SERIAL).lock(|ser| {
                write!(ser.as_mut().unwrap(), $($b)*).unwrap();
            });
        }
    }
}

//==================================================================================//
//================================🄱🅄🄸🄻🅃🄸🄽🅂==================================//
//==================================================================================//
/// Detienen el programa por una cantidad de milisegundos.
pub fn delay(duration: Duration) {
    hw(|hw| hw.start_delay(Ticks::from_duration(duration)));

    while !hw(Hw::delay_finished) {
        continue;
    }
}

pub const fn tick_count_for(duration: Duration) -> usize {
    duration.as_millis() as usize * 10
}

pub fn with_display<F, R>(callback: F) -> R
where
    F: FnOnce(&mut Display) -> R,
{
    hw(|hw| callback(&mut hw.states))
}

//==================================================================================//
//===========================🅂🄸🅂🅃🄴🄼🄰 🄴🄼🄿🄾🅃🅁🄰🄳🄾======================//
//==================================================================================//
//Descripción de sistema MCU + Periféricos
struct Hw {
    //d1: gpio::Gpio5<Output<PushPull>>,
    d4: gpio::Gpio2<Output<PushPull>>,
    d7: gpio::Gpio13<Output<PushPull>>,
    //d8: gpio::Gpio15<Output<PushPull>>,
    col_datapin: gpio::Gpio4<Output<PushPull>>,   //d2
    col_clockpin: gpio::Gpio0<Output<PushPull>>,  //d3
    row_datapin: gpio::Gpio14<Output<PushPull>>,  //d5
    row_clockpin: gpio::Gpio12<Output<PushPull>>, //d6
    states: Display,
    current_state: usize,
    timeout: Ticks,
    draw_clock: Ticks,
}

impl Hw {
    const DRAW_TICKS: Ticks = Ticks::from_duration(Duration::from_millis(3));

    fn tick(&mut self) {
        self.timeout.countdown();
        if self.draw_clock.cycle_each(Self::DRAW_TICKS) {
            self.draw();
        }
    }

    fn draw(&mut self) {
        let row_data = !(0b10000000 >> self.current_state);
        let col_data = self.states.row_bits(self.current_state) as usize;

        shift(row_data, &mut self.row_clockpin, &mut self.row_datapin);
        shift(col_data, &mut self.col_clockpin, &mut self.col_datapin);

        self.current_state += 1;
        //cambiar a 8 cuando ya no tenga que probar con un solo registro
        if self.current_state == 2 {
            self.current_state = 0;
        }
    }

    //======================timer functions====================
    fn start_delay(&mut self, timeout: Ticks) {
        self.timeout = timeout;
    }

    fn delay_finished(&mut self) -> bool {
        self.timeout.done()
    }
}

/// Punto de entrada para ESP8266.
#[entry]
fn main() -> ! {
    // Se descomponen estructuras de periféricos para formar self::HW
    let peripherals = Peripherals::take().unwrap();
    let gpio = peripherals.GPIO.split();
    let serial = peripherals
        .UART0
        .serial(gpio.gpio1.into_uart(), gpio.gpio3.into_uart());

    (&SERIAL).lock(|x| *x = Some(serial));

    {
        let hw = Hw {
            //d1: gpio.gpio5.into_push_pull_output(),
            d4: gpio.gpio2.into_push_pull_output(),
            d7: gpio.gpio13.into_push_pull_output(),
            //d8: gpio.gpio15.into_push_pull_output(),
            col_datapin: gpio.gpio4.into_push_pull_output(), //d2
            col_clockpin: gpio.gpio0.into_push_pull_output(), //d3
            row_datapin: gpio.gpio14.into_push_pull_output(), //d5
            row_clockpin: gpio.gpio12.into_push_pull_output(), //d6
            states: Default::default(),
            current_state: 0,
            timeout: Default::default(),
            draw_clock: Default::default(),
        };

        // Esto no puede escribirse con hw() debido al unwrap
        (&HW).lock(|hardware| *hardware = Some(hw));
    }

    // HEAP allocation
    unsafe {
        let start = &_heap_start as *const u8;
        let end = &_heap_end as *const u8;

        HEAP_ALLOCATOR
            .lock()
            .init(start as usize, end.offset_from(start) as usize);
    }

    let timer = unsafe { &*TIMER::ptr() };
    let dport = unsafe { &*DPORT::ptr() };

    timer.frc1_ctrl.write(|w| {
        w.timer_enable()
            .set_bit()
            .interrupt_type()
            .edge()
            .prescale_divider()
            .devided_by_1()
            .rollover()
            .set_bit()
    });

    dport
        .edge_int_enable
        .modify(|_, w| w.timer1_edge_int_enable().set_bit());

    timer.frc1_load.write(|w| unsafe { w.bits(8000) }); //pasos de 100us
    enable_interrupt(InterruptType::TIMER1);

    hw(|hw| {
        hw.d4.set_high().unwrap();
        hw.d7.set_low().unwrap();
    });

    loop {
        delay(Duration::from_secs(1));
    }

    //crate::handover();

    // Aquí no hay un sistema operativo que se encargue de hacer algo
    // cuando un progrma finaliza, por lo cual eso no puede pasar
    //panic!("user_main() returned")
}

fn shift<Clock, Data>(data: usize, clock_pin: &mut Clock, data_pin: &mut Data)
where
    Data: OutputPin<Error = Infallible>,
    Clock: OutputPin<Error = Infallible>,
{
    digital_write(clock_pin, 0);

    for i in 0..9 {
        //escribe un bit adicional para limpiar
        digital_write(clock_pin, 1);
        digital_write(data_pin, (data >> i) & 1);
        digital_write(clock_pin, 0);
    }
}

fn digital_write<Pin>(pin: &mut Pin, value: usize)
where
    Pin: OutputPin<Error = Infallible>,
{
    if value != 0 {
        pin.set_high().unwrap();
    } else {
        pin.set_low().unwrap();
    }
}

#[interrupt(TIMER1)]
#[ram]
fn timer1() {
    hw(Hw::tick);
}

/// Algo salió mal.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut x = 0;
    loop {
        x += 1;
        if x > 100_000_000 {
            x = 0;
            sys_debug!(
                "\r\n-----------Panic cause---------- \n{}\r\n-----This message repeats-----\n",
                info
            );
        }
    }
}

fn hw<F, R>(callback: F) -> R
where
    F: FnOnce(&mut Hw) -> R,
{
    (&HW).lock(|hw| callback(hw.as_mut().unwrap()))
}
