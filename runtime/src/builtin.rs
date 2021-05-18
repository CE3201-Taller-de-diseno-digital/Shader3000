#[no_mangle]
pub extern "C" fn builtin_debug() {
    crate::sys::debug();
}
