//! Implementación de `runtime::sys` cuando se dispone de
//! un sistema operativo huésped.
//!
//! Naturalmente, esto es mucho más sencillo y trivial que
//! implementar las mismas operaciones para plataformas
//! embebidas y `#![no_std]`.

/// Imprime un mensaje de depuración.
pub fn debug(hint: usize) {
    dbg!(hint);
}

/// Detiene el programa durante una cantidad de milisegundos.
pub fn delay_ms(millis: u32) {
    std::thread::sleep(std::time::Duration::from_millis(millis as u64));
}
