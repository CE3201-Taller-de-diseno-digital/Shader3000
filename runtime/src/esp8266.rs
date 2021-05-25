use esp8266_hal::{
    ehal::{blocking::delay::DelayMs, digital::v2::OutputPin},
    entry,
    gpio::{self, Output, PushPull},
    target::Peripherals,
    timer::{Timer1, TimerExt},
};

use core::convert::Infallible;

pub fn debug(hint: usize) {
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

pub fn delay_ms(millis: u32) {
    unsafe {
        HW.as_mut().unwrap().timer1.delay_ms(millis);
    }
}

struct Hw {
    gpio0: gpio::Gpio0<Output<PushPull>>,
    gpio2: gpio::Gpio2<Output<PushPull>>,
    timer1: Timer1,
}

static mut HW: Option<Hw> = None;

#[entry]
fn main() -> ! {
    use gpio::GpioExt;

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
    panic!("user_main() returned")
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        //TODO
    }
}
