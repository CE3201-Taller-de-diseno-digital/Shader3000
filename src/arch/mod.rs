use crate::{
    codegen::Context,
    ir::{Function, Global, Instruction, Local},
};

use std::io;

#[derive(Copy, Clone)]
pub enum Arch {
    X86_64,
    Xtensa,
}

mod x86_64;
mod xtensa;

pub use x86_64::Emitter as X86_64;
pub use xtensa::Emitter as Xtensa;

pub trait Emitter<'a>: Sized {
    const VALUE_SIZE: u32;

    type Register: Register;

    fn new(cx: Context<'a, Self>, instructions: &[Instruction]) -> io::Result<Self>;

    fn epilogue(self) -> io::Result<()>;
    fn cx(&mut self) -> &mut Context<'a, Self>;

    fn jump_unconditional(&mut self, label: &str) -> io::Result<()>;
    fn jump_if_false(&mut self, local: Local, label: &str) -> io::Result<()>;
    fn load_const(&mut self, value: i32, local: Local) -> io::Result<()>;
    fn load_global(&mut self, global: &Global, local: Local) -> io::Result<()>;
    fn store_global(&mut self, local: Local, global: &Global) -> io::Result<()>;

    fn call(
        &mut self,
        target: &Function,
        arguments: &[Local],
        output: Option<Local>,
    ) -> io::Result<()>;
}

pub trait Register: Copy {}
