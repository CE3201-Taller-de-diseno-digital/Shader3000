use esp8266_hal::{
    ehal::blocking::delay::DelayMs,
    entry,
    target::Peripherals,
    timer::{Timer1, TimerExt},
};

use spin::{Lazy, Mutex};

//FIXME
mod hacks {
    mod atomic;
}

pub fn debug(_hint: usize) {
    todo!()
}

pub fn delay_ms(millis: u32) {
    HW.lock().timer1.delay_ms(millis);
}

struct Hw {
    timer1: Timer1,
}

static HW: Lazy<Mutex<Hw>> = Lazy::new(|| {
    let peripherals = Peripherals::take().unwrap();
    let (timer1, _) = peripherals.TIMER.timers();

    Mutex::new(Hw { timer1 })
});

#[entry]
fn main() -> ! {
    crate::handover();
    panic!("user_main() returned")
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        //TODO
    }
}
