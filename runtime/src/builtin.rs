#[no_mangle]
pub extern "C" fn builtin_zero() -> usize {
    0
}

#[no_mangle]
pub extern "C" fn builtin_inc(n: usize) -> usize {
    n.wrapping_add(1)
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
