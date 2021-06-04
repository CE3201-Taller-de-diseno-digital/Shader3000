//! Biblioteca de soporte para CE3104 AnimationLed.
//!
//! # Propósito
//! El lenguaje especificado incluye varias funciones y funcionalidades
//! que requieren una implementación elaborada y no realizable directamente
//! por el compilador. Esta biblioteca implementa estos aspectos, así
//! como todo lo necesario para la correcta ejecución de programas emitidos
//! por el compilador.
//!
//! # Enlazado
//! El proceso de construcción de esta biblioteca requiere algunos ajustes
//! especiales con tal de poder exportar un punto de entrada para ejecutables
//! al mismo tiempo que no es en sí un ejecutable. Además, `libruntime` espera
//! en tiempo enlazado la presencia del símbolo `user_main()`, el cual debe
//! ser emitido por el compilador y es el verdadero punto de entrada del programa.
//!
//! # Uso
//! `libruntime` exporta símbolos "unmangled" usando la convención de llamada
//! que use el lenguaje C en la plataforma objetivo. Es decir, el compilador no
//! necesita emitir código Rust para usar la biblioteca, sino que es suficiente
//! con conocer el símbolo de cada función, parámetros esperados y tipo de retorno.
//! Cometer una equivocación en la forma de invocar a una función preconstruida
//! resulta en Comportamiento Indefinido y puede ocasionar problemas muy difíciles
//! de depurar.
//!
//! # Espacios de nombres
//! Las funciones preconstruidas o builtins constituyen la enteridad de
//! la interfaz pública de esta biblioteca. Todas son funciones libres
//! cuyos nombres inician con `builtin_`. Para evitar choques de símbolos
//! durante la fase de enlazado, todo símbolo emitido por el compilador
//! debe iniciar análogamente con el prefijo `user_`.
//!
//! # Toma de control
//! Es posible utilizar la biblioteca desde Rust para propósitos de
//! prueba. Ello requiere definir `#[no_mangle] extern "C" fn user_main() {}`
//! e invocar a [`handover()`].


#![cfg_attr(target_arch = "xtensa", no_std, feature(default_alloc_error_handler))]

pub mod builtin;

#[cfg(target_family = "unix")]
mod hosted;

#[cfg(target_arch = "xtensa")]
mod esp8266;

#[cfg(target_family = "unix")]
use crate::hosted as sys;

#[cfg(target_arch = "xtensa")]
use crate::esp8266 as sys;

/// Transfiere control al programa.
///
/// Esta función es el mecanismo seguro para iniciar el programa que enlazó
/// contra `libruntime`.
#[no_mangle]
pub fn handover() {
    extern "C" {
        fn user_main();
    }

    unsafe {
        user_main();
    }
}
