//! Implementación de `runtime::sys` cuando se dispone de
//! un sistema operativo huésped.
//!
//! Naturalmente, esto es mucho más sencillo y trivial que
//! implementar las mismas operaciones para plataformas
//! embebidas y `#![no_std]`.

use crate::{chrono::Duration, matrix::Display};

/// Imprime un mensaje de depuración.
macro_rules! sys_debug {
    ($($b:tt)*) => {
        println!($($b)*)
    }
}

/// Detiene el programa durante una cantidad de tiempo.
pub fn delay(duration: Duration) {
    std::thread::sleep(duration);
}

pub const fn tick_count_for(duration: Duration) -> usize {
    duration.as_millis() as usize / 10
}

pub fn with_display<F, R>(callback: F) -> R
where
    F: FnOnce(&mut Display) -> R,
{
    todo!()
}
