//! Implementación de `runtime::sys` cuando se dispone de
//! un sistema operativo huésped.
//!
//! Naturalmente, esto es mucho más sencillo y trivial que
//! implementar las mismas operaciones para plataformas
//! embebidas y `#![no_std]`.
macro_rules! debug{
    ($($b:tt)*)=>{
       {
           println!($($b)*).unwrap()
       }
    }
}
/// Imprime un mensaje de depuración.
pub fn debug(hint: usize) {
    dbg!(hint);
}
/// Detiene el programa durante una cantidad de milisegundos.
pub fn delay_ms(millis: u32) {
    std::thread::sleep(std::time::Duration::from_millis(millis as u64));
}
pub enum Interval {
    Milliseconds,
    Seconds,
    Minutes,
}
pub fn blink(_row: usize, _col: usize, _cond: bool, _interval: Interval) {
    ()
}
pub fn print_led(_col: usize, _row: usize, _value: bool) {
    ()
}
pub fn print_ledx_f(_row: usize, _value: usize) {
    ()
}
pub fn print_ledx_c(_col: usize, _value: usize) {
    ()
}
