use crate::{
    codegen::{emit_label, label_symbol},
    ir::{Function, FunctionBody, Global, Instruction, Local},
};

use std::{
    fmt,
    io::{self, Write},
    ops::Deref,
};

// Esta es una arquitectura de 64 bits
const VALUE_SIZE: u32 = 8;

pub struct Target;

impl super::Target for Target {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Register = Reg;

    fn emit_function<W: Write>(output: &mut W, function: &Function) -> io::Result<()> {
        let x86_function = X86Function { output, function };
        x86_function.write_asm()
    }
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

struct X86Function<'a, W> {
    output: &'a mut W,
    function: &'a Function,
}

impl<W: Write> X86Function<'_, W> {
    fn write_asm(mut self) -> io::Result<()> {
        writeln!(self.output, "{}:", self.function.name)?;

        let (inner_locals, instructions) = match &self.function.body {
            FunctionBody::Generated {
                inner_locals,
                instructions,
            } => (inner_locals, instructions),
            _ => return Ok(()),
        };

        // Prólogo, crea un stack frame
        emit!(self, "push", "%rbp")?;
        emit!(self, "mov", "%rsp, %rbp")?;

        // Se reserva memoria para locales
        let total_locals = self.function.parameters + inner_locals;
        let stack_allocation = total_locals + alignment_for(total_locals);
        if stack_allocation > 0 {
            self.move_rsp(-(stack_allocation as i32))?;
        }

        // Se copian argumentos de registros a locales
        for (register, local) in Reg::argument_sequence().zip(0..self.function.parameters) {
            self.register_to_local(register, Local(local))?;
        }

        // Se emite el cuerpo de la función
        for instruction in instructions {
            self.put_instruction(instruction)?;
        }

        // Epílogo, revierte al estado justo antes de la llamada
        emit!(self, "mov", "%rbp, %rsp")?;
        emit!(self, "pop", "%rbp")?;
        emit!(self, "ret")
    }

    fn put_instruction(&mut self, instruction: &Instruction) -> io::Result<()> {
        use Instruction::*;

        match instruction {
            Label(label) => emit_label(self.output, self.function, *label),
            Jump(label) => emit!(self, "jmp", "{}", label_symbol(self.function, *label)),
            JumpIfFalse(local, label) => {
                self.local_to_register(*local, Reg::Rax)?;
                emit!(self, "testl", "%eax, %eax")?;
                emit!(self, "jz", "{}", label_symbol(self.function, *label))
            }

            LoadGlobal(global, local) => {
                let Global(global) = global.deref();
                emit!(self, "mov", "{}(%rip), %rax", global)?;
                self.register_to_local(Reg::Rax, *local)
            }

            StoreGlobal(local, global) => {
                let Global(global) = global.deref();
                self.local_to_register(*local, Reg::Rax)?;
                emit!(self, "mov", "%rax, {}(%rip)", global)
            }

            Call {
                target,
                arguments,
                output: output_local,
            } => self.call(&target, &arguments, *output_local),
        }
    }

    fn call(
        &mut self,
        target: &Function,
        arguments: &[Local],
        output_local: Option<Local>,
    ) -> io::Result<()> {
        // Argumentos del séptimo en adelante se colocan en stack en orden inverso
        let pushed = (arguments.len() as u32).max(Reg::MAX_ARGS) - Reg::MAX_ARGS;
        for argument in arguments.iter().rev().take(pushed as usize) {
            emit!(self, "push", "{}", self.local_address(*argument))?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, register) in arguments.iter().zip(Reg::argument_sequence()) {
            self.local_to_register(*argument, register)?;
        }

        // Corrección del stack pointer alrededor de la llamada, manteniendo el alineamiento de 16 bytes
        let rsp_offset_after_call = if arguments.len() as u32 > Reg::MAX_ARGS {
            let alignment = alignment_for(pushed);
            if alignment > 0 {
                self.move_rsp(-(alignment as i32))?;
            }

            pushed + alignment
        } else {
            0
        };

        emit!(self, "call", "{}", target.name)?;
        if let Some(output_local) = output_local {
            self.register_to_local(Reg::Rax, output_local)?;
        }

        // Se reclama memoria que fue usada para argumentos
        if rsp_offset_after_call > 0 {
            self.move_rsp(rsp_offset_after_call as i32)?;
        }

        Ok(())
    }

    fn local_to_register(&mut self, local: Local, register: Reg) -> io::Result<()> {
        emit!(self, "mov", "{}, %{}", self.local_address(local), register)
    }

    fn register_to_local(&mut self, register: Reg, local: Local) -> io::Result<()> {
        emit!(self, "mov", "%{}, {}", register, self.local_address(local))
    }

    fn move_rsp(&mut self, offset: i32) -> io::Result<()> {
        let instruction = if offset < 0 { "subq" } else { "addq" };
        let offset = offset.abs() * VALUE_SIZE as i32;
        emit!(self, instruction, "$0x{:x}, %rsp", offset)
    }

    fn local_address(&self, Local(local): Local) -> String {
        let parameters = self.function.parameters;
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
