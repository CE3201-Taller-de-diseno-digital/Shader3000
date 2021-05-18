use esp8266_hal::entry;

//FIXME
mod hacks {
    mod atomic;
}

pub fn debug(_hint: usize) {
    todo!()
}

pub fn delay_ms(millis: u32) {
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
