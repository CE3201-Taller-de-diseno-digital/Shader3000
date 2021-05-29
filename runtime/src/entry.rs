//! Punto de entrada auxiliar para plataformas hosted.
//!
//! Las razones para la existencia de este módulo son oscuras
//! y determinarlas se deja como ejercicio para el lector.

#![crate_type = "bin"]

#[cfg(not(target_family = "unix"))]
error!("Unsupported target for entry point");

fn main() {
    extern "Rust" {
        fn handover();
    }

    unsafe {
        handover();
    }
}
