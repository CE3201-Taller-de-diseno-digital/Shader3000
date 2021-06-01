//! Compilador para CE3104 AnimationLed.
//!
//! # Front end
//! Cada programa deriva de un único archivo de código fuente.
//! Este archivo se somete primero a análisis léxico en [`lex`], de
//! lo cual se obtiene un flujo de tokens. El flujo de tokens se
//! dispone en un AST por medio de análisis sintáctico en [`parse`].
//! El árbol sintáctico es procesado por análisis semántico en
//! [`semantic`], de lo cual eventualmente se genera una representación
//! intermedia descrita en [`ir`], con lo cual concluyen las fases
//! delanteras del compilador.
//!
//! # Back end
//! En esta sección el compilador deja de ser agnóstico al sistema
//! objetivo. Es en este segmento donde ocurre generación de código
//! ensamblador y asignación de registros a variables en [`target`],
//! disposición de listados de instrucciones, ABIs y convenciones de
//! llamada en las distintas implementaciones por arquitectura,
//! concluyendo con ensamblado, enlazado y emisión del ejecutable final
//! en [`link`]. Los aspectos de ensamblado y enlazado se delegan
//! a la toolchain de `binutils` que distribuye Espressif.

#![feature(trait_alias)]

#[macro_use]
mod macros;

pub mod error;
pub mod ir;
pub mod lex;
pub mod link;
pub mod parse;
pub mod semantic;
pub mod source;

mod arch;
mod codegen;

/// Emisión de código.
///
/// Este módulo reexporta suficientes ítems internos relacionados a generación de código para
/// traducir IR a alguna arquitectura en específico.
pub mod target {
    pub use crate::arch::Arch;
    pub use crate::codegen::emit;
}
