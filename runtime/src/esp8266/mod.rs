//! Implementación de `runtime::sys` para Espressif ESP8266.
//!
//! De momento, la ausencia de atómicos funcionales en `rust-xtensa`
//! obliga a utilizar más `unsafe` de lo ideal. Como esta es una
//! plataforma `#![no_std]`, este módulo debe implementar un punto
//! de entrada específico a la plataforma y un panic handler.
extern crate xtensa_lx_rt;
use core::{convert::Infallible, fmt::Write};
use esp8266_hal::prelude::*;
use esp8266_hal::{
    ehal::digital::v2::OutputPin,
    gpio::{self, Output, PushPull},
    interrupt::*,
    target::{Peripherals, DPORT, TIMER},
    timer::{Timer1, Timer2, TimerExt},
    uart::{UART0Ext, UART0Serial},
};
use xtensa_lx::mutex::{CriticalSectionMutex, Mutex, SpinLockMutex};
mod atomic;

pub fn digital_write<Pin>(pin: &mut Pin, value: usize)
where
    Pin: OutputPin<Error = Infallible>,
{
    if value != 0 {
        pin.set_high().unwrap();
    } else {
        pin.set_low().unwrap();
    }
}

/// Detieen el programa por una cantidad de milisegundos.
pub fn delay_ms(mut millis: u32) {
    (&HW).lock(|hw| hw.as_mut().unwrap().timeout = millis);
    let mut finished = false;
    while (!finished) {
        (&HW).lock(|hw| finished = hw.as_mut().unwrap().delay_finished());
    }
}

/// Muestra información de depuración de alguna manera.
pub fn debug(hint: usize) {
    //TODO: Esto no se puede quedar así
    (&HW).lock(|hw| {
        let hw = hw.as_mut().unwrap();
        //digital_write(&mut hw.d1, hint & 0b01);
        //digital_write(&mut hw.d4, hint & 0b10);
        digital_write(&mut hw.d7, hint & 0b10)
    });
}

//Descripción de sistema MCU + Periféricos
struct Hw {
    d1: gpio::Gpio5<Output<PushPull>>,
    d4: gpio::Gpio2<Output<PushPull>>,
    d7: gpio::Gpio13<Output<PushPull>>,
    d8: gpio::Gpio15<Output<PushPull>>,
    //timer1: Timer1,
    //timer2: Timer2,
    selector_data: usize,
    col_datapin: gpio::Gpio4<Output<PushPull>>,   //d2
    col_clockpin: gpio::Gpio0<Output<PushPull>>,  //d3
    row_datapin: gpio::Gpio14<Output<PushPull>>,  //d5
    row_clockpin: gpio::Gpio12<Output<PushPull>>, //d6
    states: [usize; 8],
    current_state: usize,
    ticks: u32,
    timeout: u32,
}
enum ShiftRegister {
    Row,
    Column,
}
impl Hw {
    pub fn shift(&mut self, data: usize, register: ShiftRegister) {
        match register {
            ShiftRegister::Column => {
                digital_write(&mut self.col_clockpin, 0);
                for i in 0..9 {
                    //escribe un bit adicional para limpiar
                    digital_write(&mut self.col_clockpin, 1);
                    digital_write(&mut self.col_datapin, (data >> i) & self.selector_data);
                    digital_write(&mut self.col_clockpin, 0);
                }
            }
            ShiftRegister::Row => {
                digital_write(&mut self.row_clockpin, 0);
                for i in 0..9 {
                    //escribe un bit adicional para limpiar
                    digital_write(&mut self.row_clockpin, 1);
                    digital_write(&mut self.row_datapin, (data >> i) & self.selector_data);
                    digital_write(&mut self.row_clockpin, 0);
                }
            }
        }
    }
    pub fn draw(&mut self) {
        &mut self.shift(!(0b10000000 >> self.current_state), ShiftRegister::Row);
        &mut self.shift(self.states[self.current_state], ShiftRegister::Column);
        &mut self.next_row();
    }
    pub fn next_row(&mut self) {
        if self.current_state < 2 {
            self.current_state += 1;
        } else {
            self.current_state = 0;
        }
    }
    pub fn draw_three(&mut self) {
        &mut self.shift(self.states[self.current_state], ShiftRegister::Column);
        &mut self.next_row();
    }
    pub fn tick(&mut self) {
        self.ticks = self.ticks + 1;
        if (self.timeout > 0) {
            self.timeout -= 1
        }
    }
    pub fn reset_ticks(&mut self) {
        self.ticks = 0;
    }
    pub fn compare_ticks(&mut self, n: u32) -> bool {
        self.ticks > n
    }
    pub fn get_ticks(&mut self) -> u32 {
        self.ticks
    }
    pub fn start_delay(&mut self, time: u32) {
        self.timeout = time;
    }
    pub fn delay_finished(&mut self) -> bool {
        self.timeout == 0
    }
}
/// Instancia global de periféricos, ya que no tenemos atómicos.
static HW: CriticalSectionMutex<Option<Hw>> = CriticalSectionMutex::new(None);
static SERIAL: CriticalSectionMutex<Option<UART0Serial>> = CriticalSectionMutex::new(None);

/// Punto de entrada para ESP8266.
#[entry]
fn main() -> ! {
    //use esp8266_hal::gpio::GpioExt;
    // Se descomponen estructuras de periféricos para formar self::HW
    let peripherals = Peripherals::take().unwrap();
    let gpio = peripherals.GPIO.split();
    let mut serial = peripherals
        .UART0
        .serial(gpio.gpio1.into_uart(), gpio.gpio3.into_uart());
    (&SERIAL).lock(|x| *x = Some(serial));
    let mut hw = Hw {
        d1: gpio.gpio5.into_push_pull_output(),
        d4: gpio.gpio2.into_push_pull_output(),
        d7: gpio.gpio13.into_push_pull_output(),
        d8: gpio.gpio15.into_push_pull_output(),
        //timer1,
        //timer2,
        selector_data: 0b00000001,
        col_datapin: gpio.gpio4.into_push_pull_output(), //d2
        col_clockpin: gpio.gpio0.into_push_pull_output(), //d3
        states: [
            0b11100111, 0b11101011, 0b11101101, 0b11111011, 0b11111101, 0b11110111, 0b11001011,
            0b11111101,
        ],
        current_state: 0,
        row_datapin: gpio.gpio14.into_push_pull_output(), //d5
        row_clockpin: gpio.gpio12.into_push_pull_output(), //d6
        ticks: 0,
        timeout: 0,
    };
    (&HW).lock(|hardware| *hardware = Some(hw));
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
    timer.frc1_load.write(|w| unsafe { w.bits(0b111111111) });
    enable_interrupt(InterruptType::TIMER1);
    (&HW).lock(|hw|{
        hw.as_mut().unwrap().d4.set_high().unwrap();
        hw.as_mut().unwrap().d7.set_low().unwrap();
    });
    let mut time = 0;
    loop {
        //delay_ms(100000);
        (&HW).lock(|hw| {
            hw.as_mut().unwrap().d7.toggle().unwrap();
            time = hw.as_mut().unwrap().get_ticks();
        });
        (&SERIAL).lock(|ser| write!(ser.as_mut().unwrap(), "\r\neeee: -{}\r\n", time).unwrap());
    }
    crate::handover();

    // Aquí no hay un sistema operativo que se encargue de hacer algo
    // cuando un progrma finaliza, por lo cual eso no puede pasar
    panic!("user_main() returned")
}

/// Algo salió mal.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        //TODO
    }
}

#[interrupt]
#[ram]
fn timer1() {
    (&HW).lock(|hw| {
        hw.as_mut().unwrap().tick();
        if hw.as_mut().unwrap().compare_ticks(10) {
            hw.as_mut().unwrap().reset_ticks();
            hw.as_mut().unwrap().d4.toggle().unwrap();
        }
    });
}
