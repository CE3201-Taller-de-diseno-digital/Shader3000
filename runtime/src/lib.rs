#![cfg_attr(target_arch = "xtensa", no_std)]

pub mod builtin;

#[cfg(target_family = "unix")]
mod hosted;

#[cfg(target_arch = "xtensa")]
mod esp8266;

#[cfg(target_family = "unix")]
use crate::hosted as sys;

#[cfg(target_arch = "xtensa")]
use crate::esp8266 as sys;

#[no_mangle]
pub extern "Rust" fn handover() {
    extern "C" {
        fn user_main();
    }

    unsafe {
        user_main();
    }
}
