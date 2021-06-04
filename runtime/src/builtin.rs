//! Interfaz pública.
//!
//! Las funciones en este módulo están diseñadas para ser
//! invocadas en la forma descrita en la documentación
//! top-level de este crate. Es necesario que todas sean
//! tanto `#[no_mangle]` como `extern "C"`.
//!
//! # Aquí no hay magia
//! Para garantizar agnosticismo en varias partes donde
//! no se necesita conocer detalles de la plataforma. En
//! vez de eso, este módulo debe realizar a llamadas a
//! `crate::sys::*` cuando se necesita una operación
//! que depende de la plataforma.

/// Retorna cero.
///
///TODO: Eliminar
#[no_mangle]
pub extern "C" fn builtin_zero() -> usize {
    0
}

/// Incrementa un entero.
///
///TODO: Eliminar
#[no_mangle]
pub extern "C" fn builtin_inc(n: usize) -> usize {
    n.wrapping_add(1)
}

/// Imprime información de epuración en alguna manera no especificada.
#[no_mangle]
pub extern "C" fn builtin_debug(hint: usize) {
    crate::sys::debug(hint);
}

/// Detiene el programa por una cantidad de milisegundos.
#[no_mangle]
pub extern "C" fn builtin_delay_mil(millis: u32) {
    crate::sys::delay_ms(millis);
}

/// Detiene el programa por una cantidad de segundos.
#[no_mangle]
pub extern "C" fn builtin_delay_seg(secs: u32) {
    crate::sys::delay_ms(secs * 1000);
}

/// Detiene el programa por una cantidad de minutos.
#[no_mangle]
pub extern "C" fn builtin_delay_min(mins: u32) {
    crate::sys::delay_ms(mins * 60000);
}

#[no_mangle]
pub extern "C" fn builtin_blink_mil(row: usize, col: usize, cond: bool) {
    crate::sys::blink(row, col,cond, crate::sys::Interval::Milliseconds);
}

#[no_mangle]
pub extern "C" fn builtin_blink_seg(row: usize, col: usize, cond: bool) {
    crate::sys::blink(row, col,cond, crate::sys::Interval::Seconds);
}

#[no_mangle]
pub extern "C" fn builtin_blink_min(row: usize, col: usize, cond: bool) {
    crate::sys::blink(row, col,cond, crate::sys::Interval::Minutes);
}

#[no_mangle]
pub extern "C" fn print_led(row: usize, col: usize, value: usize) {
    crate::sys::print_led(row, col, value);
}
