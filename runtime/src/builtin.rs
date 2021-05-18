#[no_mangle]
pub extern "C" fn builtin_true() -> usize {
    true as usize
}

#[no_mangle]
pub extern "C" fn builtin_false() -> usize {
    false as usize
}

#[no_mangle]
pub extern "C" fn builtin_neg(boolean: usize) -> usize {
    !(boolean & 1 == 1) as usize
}

#[no_mangle]
pub extern "C" fn builtin_debug(hint: usize) {
    crate::sys::debug(hint);
}

#[no_mangle]
pub extern "C" fn builtin_delay_mil(millis: u32) {
    crate::sys::delay_ms(millis);
}

#[no_mangle]
pub extern "C" fn builtin_delay_seg(secs: u32) {
    crate::sys::delay_ms(secs * 1000);
}

#[no_mangle]
pub extern "C" fn builtin_delay_min(mins: u32) {
    crate::sys::delay_ms(mins * 60000);
}
