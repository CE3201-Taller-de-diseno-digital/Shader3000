use crate::{
    codegen::Context,
    ir::{Function, Global, Instruction, Local},
};

use std::{
    fmt,
    io::{self, Write},
};

// Esta es una arquitectura de 64 bits
const VALUE_SIZE: u32 = 8;

pub struct Target;

impl super::Target for Target {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Emitter = Emitter;
    type Register = Reg;
}

#[derive(Copy, Clone)]
pub enum Reg {
    Rax,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    R8,
    R9,
}

impl Reg {
    /* La ABI indica que se coloquen los primeros 6 argumentos en los registros %rdi, %rsi, %rdx, %rcx,
     * %r8 y %r9. Si hay más se ponen en el stack en orden inverso.
     */
    const MAX_ARGS: u32 = 6;

    fn argument_sequence() -> impl Iterator<Item = Reg> {
        use Reg::*;

        std::iter::successors(Some(Rdi), |last| match last {
            Rdi => Some(Rsi),
            Rsi => Some(Rdx),
            Rdx => Some(Rcx),
            Rcx => Some(R8),
            R8 => Some(R9),
            _ => None,
        })
    }
}

impl super::Register for Reg {}

impl fmt::Display for Reg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Reg::*;

        let name = match self {
            Rax => "rax",
            Rcx => "rcx",
            Rdx => "rdx",
            Rsi => "rsi",
            Rdi => "rdi",
            R8 => "r8",
            R9 => "r9",
        };

        formatter.write_str(name)
    }
}

pub struct Emitter;

impl super::Emitter for Emitter {
    fn new(_: &[Instruction]) -> Self {
        Emitter
    }

    fn prologue<W: Write>(&mut self, cx: &mut Context<W>) -> io::Result<()> {
        // Se crea un stack frame
        emit!(cx, "push", "%rbp")?;
        emit!(cx, "mov", "%rsp, %rbp")?;

        // Se reserva memoria para locales
        let total_locals = cx.agnostic_locals();
        let stack_allocation = total_locals + alignment_for(total_locals);

        if stack_allocation > 0 {
            self.move_rsp(cx, -(stack_allocation as i32))?;
        }

        // Se copian argumentos de registros a locales
        for (register, local) in Reg::argument_sequence().zip(0..cx.function().parameters) {
            self.register_to_local(cx, register, Local(local))?;
        }

        Ok(())
    }

    fn epilogue<W: Write>(&mut self, cx: &mut Context<W>) -> io::Result<()> {
        // Revierte al estado justo antes de la llamada
        emit!(cx, "mov", "%rbp, %rsp")?;
        emit!(cx, "pop", "%rbp")?;
        emit!(cx, "ret")
    }

    fn jump_unconditional<W: Write>(&mut self, cx: &mut Context<W>, label: &str) -> io::Result<()> {
        emit!(cx, "jmp", "{}", label)
    }

    fn jump_if_false<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        label: &str,
    ) -> io::Result<()> {
        self.local_to_register(cx, local, Reg::Rax)?;
        emit!(cx, "testl", "%eax, %eax")?;
        emit!(cx, "jz", "{}", label)
    }

    fn load_global<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        Global(global): &Global,
        local: Local,
    ) -> io::Result<()> {
        emit!(cx, "mov", "{}(%rip), %rax", global)?;
        self.register_to_local(cx, Reg::Rax, local)
    }

    fn store_global<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        Global(global): &Global,
    ) -> io::Result<()> {
        self.local_to_register(cx, local, Reg::Rax)?;
        emit!(cx, "mov", "%rax, {}(%rip)", global)
    }

    fn call<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        target: &Function,
        arguments: &[Local],
        output_local: Option<Local>,
    ) -> io::Result<()> {
        // Argumentos del séptimo en adelante se colocan en stack en orden inverso
        let pushed = (arguments.len() as u32).max(Reg::MAX_ARGS) - Reg::MAX_ARGS;
        for argument in arguments.iter().rev().take(pushed as usize) {
            let address = self.local_address(cx, *argument);
            emit!(cx, "push", "{}", address)?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, register) in arguments.iter().zip(Reg::argument_sequence()) {
            self.local_to_register(cx, *argument, register)?;
        }

        // Corrección del stack pointer alrededor de la llamada, manteniendo el alineamiento de 16 bytes
        let rsp_offset_after_call = if arguments.len() as u32 > Reg::MAX_ARGS {
            let alignment = alignment_for(pushed);
            if alignment > 0 {
                self.move_rsp(cx, -(alignment as i32))?;
            }

            pushed + alignment
        } else {
            0
        };

        emit!(cx, "call", "{}", target.name)?;
        if let Some(output_local) = output_local {
            self.register_to_local(cx, Reg::Rax, output_local)?;
        }

        // Se reclama memoria que fue usada para argumentos
        if rsp_offset_after_call > 0 {
            self.move_rsp(cx, rsp_offset_after_call as i32)?;
        }

        Ok(())
    }
}

impl Emitter {
    fn local_to_register<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        register: Reg,
    ) -> io::Result<()> {
        let address = self.local_address(cx, local);
        emit!(cx, "mov", "{}, %{}", address, register)
    }

    fn register_to_local<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        register: Reg,
        local: Local,
    ) -> io::Result<()> {
        let address = self.local_address(cx, local);
        emit!(cx, "mov", "%{}, {}", register, address)
    }

    fn move_rsp<W: Write>(&mut self, cx: &mut Context<W>, offset: i32) -> io::Result<()> {
        let instruction = if offset < 0 { "subq" } else { "addq" };
        let offset = offset.abs() * VALUE_SIZE as i32;
        emit!(cx, instruction, "$0x{:x}, %rsp", offset)
    }

    fn local_address<W: Write>(&self, cx: &Context<W>, Local(local): Local) -> String {
        let parameters = cx.function().parameters;
        let value_offset = if local < Reg::MAX_ARGS || parameters < Reg::MAX_ARGS {
            -1 - local as i32
        } else if local < parameters {
            1 + (local - Reg::MAX_ARGS) as i32
        } else {
            -1 - (Reg::MAX_ARGS + local - parameters) as i32
        };

        let offset = value_offset * (VALUE_SIZE as i32);
        let sign = if offset < 0 { "-" } else { "" };
        format!("{}0x{:x}(%rbp)", sign, offset.abs())
    }
}

fn alignment_for(pushed: u32) -> u32 {
    // Cada valor es de 64 bits (8 bytes), y la frontera de alineamiento es de 16 bytes
    pushed % 2
}
