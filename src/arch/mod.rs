use crate::{
    codegen::Context,
    ir::{Function, Global, Instruction, Local},
};

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

    type Emitter: Emitter;
    type Register: Register;
}

pub trait Emitter {
    fn new(instructions: &[Instruction]) -> Self;

    fn prologue<W: Write>(&mut self, cx: &mut Context<W>) -> io::Result<()>;
    fn epilogue<W: Write>(&mut self, cx: &mut Context<W>) -> io::Result<()>;

    fn jump_unconditional<W: Write>(&mut self, cx: &mut Context<W>, label: &str) -> io::Result<()>;

    fn jump_if_false<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        label: &str,
    ) -> io::Result<()>;

    fn load_global<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        global: &Global,
        local: Local,
    ) -> io::Result<()>;

    fn store_global<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        global: &Global,
    ) -> io::Result<()>;

    fn call<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        target: &Function,
        arguments: &[Local],
        output: Option<Local>,
    ) -> io::Result<()>;
}

pub trait Register: Copy {}
