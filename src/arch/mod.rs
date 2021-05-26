use crate::ir::Function;
use std::{
    io::{self, Write},
    str::FromStr,
};

#[derive(Copy, Clone)]
pub enum Arch {
    X86_64,
    Xtensa,
}

impl FromStr for Arch {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "x86_64" => Ok(Arch::X86_64),
            "xtensa" => Ok(Arch::Xtensa),
            _ => Err(()),
        }
    }
}

mod x86_64;
mod xtensa;

pub use x86_64::Target as X86_64;
pub use xtensa::Target as Xtensa;

pub trait Target {
    const VALUE_SIZE: u32;

    type Register: Register;

    fn emit_function<W: Write>(output: &mut W, function: &Function) -> io::Result<()>;
}

pub trait Register: Copy {}
