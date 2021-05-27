#![feature(trait_alias)]

#[macro_use]
mod macros;

pub mod ir;
pub mod lex;
pub mod parse;
pub mod source;

mod arch;
mod codegen;

pub mod target {
    pub use crate::arch::Arch;
    pub use crate::codegen::emit;
}
