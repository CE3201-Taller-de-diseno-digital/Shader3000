//! Implementación de `runtime::sys` para Espressif ESP8266.
//!
//! De momento, la ausencia de atómicos funcionales en `rust-xtensa`
//! obliga a utilizar más `unsafe` de lo ideal. Como esta es una
//! plataforma `#![no_std]`, este módulo debe implementar un punto
//! de entrada específico a la plataforma y un panic handler.

use core::convert::Infallible;
use esp8266_hal::{
    prelude::*,
    ehal::{blocking::delay::{DelayMs,DelayUs}, digital::v2::OutputPin, timer::{Cancel, CountDown, Periodic}},
    entry,
    gpio::{self, Output, PushPull},
    target::Peripherals,
    timer::{Timer1, Timer2, TimerExt},
};

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
        unsafe {
            HW.as_mut().unwrap().timer1.delay_ms(delay);
        }
        millis -= delay;
    }
}
/// Detieen el programa por una cantidad de microsegundos.
pub fn delay_us(mut micros: u32) {
    while micros > 0 {
        let delay = micros.min(1000);
        unsafe {
            HW.as_mut().unwrap().timer1.delay_us(delay);
        }
        micros -= delay;
    }
}
/// Muestra información de depuración de alguna manera.
pub fn debug(hint: usize) {
    //TODO: Esto no se puede quedar así
    unsafe {
        let hw = HW.as_mut().unwrap();
        digital_write(&mut hw.d1, hint & 0b01);
        digital_write(&mut hw.d4, hint & 0b10);
        digital_write(&mut hw.d7, hint & 0b10);
    }
}

//Descripción de sistema MCU + Periféricos
struct Hw {
    d1: gpio::Gpio5<Output<PushPull>>,
    //d2: gpio::Gpio4<Output<PushPull>>,
    //d3: gpio::Gpio0<Output<PushPull>>,
    d4: gpio::Gpio2<Output<PushPull>>,
    //d5: gpio::Gpio14<Output<PushPull>>,
    //d6: gpio::Gpio12<Output<PushPull>>,
    d7: gpio::Gpio13<Output<PushPull>>,
    d8: gpio::Gpio15<Output<PushPull>>,
    timer1: Timer1,
    timer2: Timer2,
    selector_data: usize,
    col_datapin: gpio::Gpio4<Output<PushPull>>,   //d2
    col_clockpin: gpio::Gpio0<Output<PushPull>>,  //d3
    row_datapin: gpio::Gpio14<Output<PushPull>>,  //d
    row_clockpin: gpio::Gpio12<Output<PushPull>>, //d
    states: [usize; 8],
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
        for i in 0..8 {
            delay_us(300);
            &mut self.shift(!(0b10000000 >> i), ShiftRegister::Row);
            &mut self.shift(self.states[i], ShiftRegister::Column);
        }
    }
    pub fn draw_three(&mut self) {
        &mut self.shift(self.states[0], ShiftRegister::Column);
        delay_us(300);
        &mut self.shift(self.states[1], ShiftRegister::Column);
        delay_us(300);
        &mut self.shift(self.states[2], ShiftRegister::Column);
        delay_us(300);
    }
}
/// Instancia global de periféricos, ya que no tenemos atómicos.
static mut HW: Option<Hw> = None;

/// Punto de entrada para ESP8266.
#[entry]
fn main() -> ! {
    use gpio::GpioExt;
    // Se descomponen estructuras de periféricos para formar self::HW
    let peripherals = Peripherals::take().unwrap();
    let gpio = peripherals.GPIO.split();
    let (timer1, timer2) = peripherals.TIMER.timers();
    unsafe {
        HW = Some(Hw {
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
                0b11100111, 0b10101011, 0b01001101, 0b01111100, 0b01000100, 0b10000010, 0b11111111,
                0b11000011,
            ],
            row_datapin: gpio.gpio14.into_push_pull_output(), //d5
            row_clockpin: gpio.gpio12.into_push_pull_output(), //d6
        });
    }

    //single_shift_test();
    draw_test();
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

//funciones de prueba de sistema

fn draw_test() {
    unsafe {
        let hw = HW.as_mut().unwrap();
        digital_write(&mut hw.d4, 1); //apaga led builtin
        let mut delay = 1000;
        loop {
            //hw.draw_three();
            hw.draw(); //testeable en matriz de leds
        }
    }
}

fn single_shift_test() {
    unsafe {
        let hw = HW.as_mut().unwrap();
        digital_write(&mut hw.d4, 1); //apaga led builtin
        loop {
            for i in 0..255 {
                hw.shift(i, ShiftRegister::Column);
                delay_ms(500);
            }
        }
    }
}

fn test1() {
    let mut x: usize = 0;
    let mut t: u32 = 0;
    loop {
        debug(x);
        x = x + 1;
        t = t + 1;
        delay_ms(t * 25);
    }
}
