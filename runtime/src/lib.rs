pub mod builtin;

#[cfg(target_family = "unix")]
mod hosted;

#[cfg(target_family = "unix")]
use crate::hosted as sys;

#[no_mangle]
pub fn handover() {
    extern "C" {
        fn user_main();
    }

    unsafe {
        user_main();
    }
}
