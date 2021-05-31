//! Implementación de `runtime::sys` para Espressif ESP8266.
//!
//! De momento, la ausencia de atómicos funcionales en `rust-xtensa`
//! obliga a utilizar más `unsafe` de lo ideal. Como esta es una
//! plataforma `#![no_std]`, este módulo debe implementar un punto
//! de entrada específico a la plataforma y un panic handler.

use core::convert::Infallible;
use esp8266_hal::{
    ehal::{blocking::delay::DelayMs, digital::v2::OutputPin},
    entry,
    gpio::{self, Output, PushPull},
    target::Peripherals,
    timer::{Timer1, Timer2, TimerExt},
};

pub struct LedMatrix {
    column_data: [[usize; 8]; 8],
    column_register: ShiftRegister,
    row_register: ShiftRegister,
}

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

pub struct ShiftRegister {
    current_data: usize,
    selector_data: usize,
}
impl ShiftRegister {
    pub fn new() -> Self {
        ShiftRegister {
            current_data: 0b0000000,
            selector_data: 0b00000001,
        }
    }
    pub fn shift<Data, Clk>(&mut self, datapin: &mut Data, clockpin: &mut Clk, data: usize)
    where
        Data: OutputPin<Error = Infallible>,
        Clk: OutputPin<Error = Infallible>,
    {
        self.current_data = data;
        digital_write(clockpin, 0);
        for i in 0..8 {
            digital_write(clockpin, 1);
            digital_write(datapin, (data >> i) & self.selector_data);
            digital_write(clockpin, 0);
        }
    }
}
/// Muestra información de depuración de alguna manera.
pub fn debug(hint: usize) {
    //TODO: Esto no se puede quedar así
    unsafe {
        let hw = NODEMCU.as_mut().unwrap();
        digital_write(&mut hw.d1, hint & 0b01);
        digital_write(&mut hw.d2, hint & 0b10);
        digital_write(&mut hw.d3, hint & 0b01);
        digital_write(&mut hw.d4, hint & 0b10);
        digital_write(&mut hw.d5, hint & 0b10);
        digital_write(&mut hw.d6, hint & 0b01);
        digital_write(&mut hw.d7, hint & 0b10);
    }
}

/// Detieen el programa por una cantidad de milisegundos.
pub fn delay_ms(mut millis: u32) {
    while millis > 0 {
        let delay = millis.min(1000);
        unsafe {
            NODEMCU.as_mut().unwrap().timer1.delay_ms(delay);
        }
        millis -= delay;
    }
}
/// Recursos de E/S y eventos ("periféricos").
struct NodeMCU {
    d1: gpio::Gpio5<Output<PushPull>>,
    d2: gpio::Gpio4<Output<PushPull>>,
    d3: gpio::Gpio0<Output<PushPull>>,
    d4: gpio::Gpio2<Output<PushPull>>,
    d5: gpio::Gpio14<Output<PushPull>>,
    d6: gpio::Gpio12<Output<PushPull>>,
    d7: gpio::Gpio13<Output<PushPull>>,
    timer1: Timer1,
    timer2: Timer2,
}
/// Instancia global de periféricos, ya que no tenemos atómicos.
static mut NODEMCU: Option<NodeMCU> = None;

/// Punto de entrada para ESP8266.
#[entry]
fn main() -> ! {
    use gpio::GpioExt;

    // Se descomponen estructuras de periféricos para formar self::HW
    let peripherals = Peripherals::take().unwrap();
    let gpio = peripherals.GPIO.split();
    let (timer1, timer2) = peripherals.TIMER.timers();

    unsafe {
        NODEMCU = Some(NodeMCU {
            d1: gpio.gpio5.into_push_pull_output(),
            d2: gpio.gpio4.into_push_pull_output(),
            d3: gpio.gpio0.into_push_pull_output(),
            d4: gpio.gpio2.into_push_pull_output(),
            d5: gpio.gpio14.into_push_pull_output(),
            d6: gpio.gpio12.into_push_pull_output(),
            d7: gpio.gpio13.into_push_pull_output(),
            timer1,
            timer2,
        });
    }
    shift_test();
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

fn shift_test() {
    let mut register = ShiftRegister::new();
    unsafe {
        let hw = NODEMCU.as_mut().unwrap();
        let datapin = &mut hw.d2;
        let clockpin = &mut hw.d3;
        digital_write(&mut hw.d4, 1);
        loop {
            register.shift(datapin, clockpin, 0b11111111);
            delay_ms(500);
            register.shift(datapin, clockpin, 0b00000000);
            delay_ms(500);
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
