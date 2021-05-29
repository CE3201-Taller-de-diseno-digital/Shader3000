//! Implementación de `runtime::sys` para Espressif ESP8266.
//!
//! De momento, la ausencia de atómicos funcionales en `rust-xtensa`
//! obliga a utilizar más `unsafe` de lo ideal. Como esta es una
//! plataforma `#![no_std]`, este módulo debe implementar un punto
//! de entrada específico a la plataforma y un panic handler.

use esp8266_hal::{
    ehal::{blocking::delay::DelayMs, digital::v2::OutputPin},
    entry,
    gpio::{self, Output, PushPull},
    target::Peripherals,
    timer::{Timer1, TimerExt},
};

use core::convert::Infallible;

/// Muestra información de depuración de alguna manera.
pub fn debug(hint: usize) {
    //TODO: Esto no se puede quedar así

    fn set<Pin>(pin: &mut Pin, condition: bool)
    where
        Pin: OutputPin<Error = Infallible>,
    {
        if condition {
            pin.set_high().unwrap();
        } else {
            pin.set_low().unwrap();
        }
    }

    unsafe {
        let hw = HW.as_mut().unwrap();
        set(&mut hw.gpio0, (hint & 0b01) != 0);
        set(&mut hw.gpio2, (hint & 0b10) != 0);
    }
}

/// Detieen el programa por una cantidad de milisegundos.
pub fn delay_ms(millis: u32) {
    unsafe {
        HW.as_mut().unwrap().timer1.delay_ms(millis);
    }
}

/// Recursos de E/S y eventos ("periféricos").
struct Hw {
    gpio0: gpio::Gpio0<Output<PushPull>>,
    gpio2: gpio::Gpio2<Output<PushPull>>,
    timer1: Timer1,
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
    let (timer1, _) = peripherals.TIMER.timers();

    unsafe {
        HW = Some(Hw {
            gpio0: gpio.gpio0.into_push_pull_output(),
            gpio2: gpio.gpio2.into_push_pull_output(),
            timer1,
        });
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
