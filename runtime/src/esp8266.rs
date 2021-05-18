use esp8266_hal::entry;

pub fn debug() {
    todo!()
}

pub fn delay_ms(_millis: u32) {
    todo!()
}

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
