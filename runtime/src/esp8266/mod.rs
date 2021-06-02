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
    target::Peripherals,
    timer::{Timer1, Timer2, TimerExt},
    uart::{UART0Ext, UART0Serial},
};
use xtensa_lx::mutex::{CriticalSectionMutex, Mutex, SpinLockMutex};

extern crate compiler_builtins;
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
    while millis > 0 {
        let delay = millis.min(1000);
        (&HW).lock(|hw| {
            let hw = hw.as_mut().unwrap();
            hw.timer2.delay_ms(delay);
        });
        millis -= delay;
    }
}
/// Detieen el programa por una cantidad de microsegundos.
pub fn delay_us(mut micros: u32) {
    while micros > 0 {
        let delay = micros.min(1000);
        (&HW).lock(|hw| {
            let hw = hw.as_mut().unwrap();
            hw.timer2.delay_us(delay);
        });
        micros -= delay;
    }
}
/// Muestra información de depuración de alguna manera.
pub fn debug(hint: usize) {
    //TODO: Esto no se puede quedar así
    (&HW).lock(|hw| {
        let hw = hw.as_mut().unwrap();
        digital_write(&mut hw.d1, hint & 0b01);
        //digital_write(&mut hw.d4, hint & 0b10);
        //digital_write(&mut hw.d7, hint & 0b10)
    });
}

//Descripción de sistema MCU + Periféricos
struct Hw {
    d1: gpio::Gpio5<Output<PushPull>>,
    d4: gpio::Gpio2<Output<PushPull>>,
    d7: gpio::Gpio13<Output<PushPull>>,
    d8: gpio::Gpio15<Output<PushPull>>,
    timer1: Timer1,
    timer2: Timer2,
    selector_data: usize,
    col_datapin: gpio::Gpio4<Output<PushPull>>,   //d2
    col_clockpin: gpio::Gpio0<Output<PushPull>>,  //d3
    row_datapin: gpio::Gpio14<Output<PushPull>>,  //d5
    row_clockpin: gpio::Gpio12<Output<PushPull>>, //d6
    states: [usize; 8],
    current_state: usize,
    serial: UART0Serial,
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
        if (self.current_state < 7) {
            self.current_state += 1;
        } else {
            self.current_state = 0;
        }
    }
    pub fn draw_three(&mut self) {
        &mut self.shift(self.states[self.current_state], ShiftRegister::Column);
        &mut self.next_row();
    }
}
/// Instancia global de periféricos, ya que no tenemos atómicos.
//static mut HW: Option<Hw> = None;
static HW: SpinLockMutex<Option<Hw>> = SpinLockMutex::new(None);
static MILLIS: CriticalSectionMutex<Option<u32>> = CriticalSectionMutex::new(None);
/// Punto de entrada para ESP8266.
#[entry]
fn main() -> ! {
    //use esp8266_hal::gpio::GpioExt;
    // Se descomponen estructuras de periféricos para formar self::HW
    let peripherals = Peripherals::take().unwrap();
    let gpio = peripherals.GPIO.split();
    let (timer1, timer2) = peripherals.TIMER.timers();
    let serial = peripherals
        .UART0
        .serial(gpio.gpio1.into_uart(), gpio.gpio3.into_uart());
    let mut hw = Hw {
        d1: gpio.gpio5.into_push_pull_output(),
        d4: gpio.gpio2.into_push_pull_output(),
        d7: gpio.gpio13.into_push_pull_output(),
        d8: gpio.gpio15.into_push_pull_output(),
        timer1,
        timer2,
        selector_data: 0b00000001,
        col_datapin: gpio.gpio4.into_push_pull_output(), //d2
        col_clockpin: gpio.gpio0.into_push_pull_output(), //d3
        states: [
            0b11010111, 0b10111101, 0b10001011, 0b11111011, 0b11111101, 0b11110111, 0b11001011,
            0b11111101,
        ],
        current_state: 0,
        row_datapin: gpio.gpio14.into_push_pull_output(), //d5
        row_clockpin: gpio.gpio12.into_push_pull_output(), //d6
        serial,                                           //serial,
    };
    (&HW).lock(|hardware| *hardware = Some(hw));
    (&HW).lock(|hw| hw.as_mut().unwrap().timer1.enable_interrupts());
    enable_interrupt(InterruptType::TIMER1);
    (&MILLIS).lock(|time| *time = Some(0));

    loop {
        //delay_ms(2);
        //(&HW).lock(|hw| hw.as_mut().unwrap().d7.toggle().unwrap());
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
static mut CL: u32 = 0;

#[interrupt]
fn timer1() {
    unsafe {
        CL += 1;
        if (CL % 50000 == 0) {
            (&HW).lock(|hw| hw.as_mut().unwrap().d4.toggle().unwrap());
        }
        if (CL % 100 == 0) {
            (&HW).lock(|hw| hw.as_mut().unwrap().draw());
        }
    }
}
