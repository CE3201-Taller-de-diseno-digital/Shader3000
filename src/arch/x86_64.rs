//! Implementación para x86-64.

use crate::{
    codegen::Context,
    ir::{Function, Global, Instruction, Local},
};

use std::{fmt, io};

/// Esta es una arquitectura de 64 bits
const VALUE_SIZE: u32 = 8;

/// Registro de procesador.
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
    /// La ABI indica que se coloquen los primeros 6 argumentos en los
    /// registros `%rdi`, `%rsi`, `%rdx`, `%rcx`, `%r8` y `%r9`. Si hay
    /// más se ponen en el stack en orden inverso.
    const MAX_ARGS: u32 = 6;

    /// Iterador sobre los registros donde se colocan los primeros argumentos.
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

/// Emisión de código para x86-64.
pub struct Emitter<'a> {
    cx: Context<'a, Self>,
}

impl<'a> super::Emitter<'a> for Emitter<'a> {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Register = Reg;

    fn new(mut cx: Context<'a, Self>, _: &[Instruction]) -> io::Result<Self> {
        // Prólogo, se crea un stack frame
        emit!(cx, "push", "%rbp")?;
        emit!(cx, "mov", "%rsp, %rbp")?;

        // Se reserva memoria para locales
        let total_locals = cx.agnostic_locals();
        let stack_allocation = total_locals + alignment_for(total_locals);

        let mut emitter = Emitter { cx };

        if stack_allocation > 0 {
            emitter.move_rsp(-(stack_allocation as i32))?;
        }

        // Se copian argumentos de registros a locales
        let parameters = emitter.cx.function().parameters;
        for (register, local) in Reg::argument_sequence().zip(0..parameters) {
            emitter.register_to_local(register, Local(local))?;
        }

        Ok(emitter)
    }

    fn cx(&mut self) -> &mut Context<'a, Self> {
        &mut self.cx
    }

    fn epilogue(mut self) -> io::Result<()> {
        // Revierte al estado justo antes de la llamada
        emit!(self.cx, "mov", "%rbp, %rsp")?;
        emit!(self.cx, "pop", "%rbp")?;
        emit!(self.cx, "ret")
    }

    fn jump_unconditional(&mut self, label: &str) -> io::Result<()> {
        emit!(self.cx, "jmp", "{}", label)
    }

    fn jump_if_false(&mut self, local: Local, label: &str) -> io::Result<()> {
        self.local_to_register(local, Reg::Rax)?;
        emit!(self.cx, "testl", "%eax, %eax")?;
        emit!(self.cx, "jz", "{}", label)
    }

    fn load_const(&mut self, value: i32, local: Local) -> io::Result<()> {
        emit!(self.cx, "mov", "${}, %rax", value)?;
        self.register_to_local(Reg::Rax, local)
    }

    fn load_global(&mut self, Global(global): &Global, local: Local) -> io::Result<()> {
        emit!(self.cx, "mov", "{}(%rip), %rax", global)?;
        self.register_to_local(Reg::Rax, local)
    }

    fn store_global(&mut self, local: Local, Global(global): &Global) -> io::Result<()> {
        self.local_to_register(local, Reg::Rax)?;
        emit!(self.cx, "mov", "%rax, {}(%rip)", global)
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
            let address = self.local_address(*argument);
            emit!(self.cx, "push", "{}", address)?;
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

        emit!(self.cx, "call", "{}", target.name)?;
        if let Some(output_local) = output_local {
            self.register_to_local(Reg::Rax, output_local)?;
        }

        // Se reclama memoria que fue usada para argumentos
        if rsp_offset_after_call > 0 {
            self.move_rsp(rsp_offset_after_call as i32)?;
        }

        Ok(())
    }
}

impl Emitter<'_> {
    /// Copia los contenidos de una local a un registro.
    fn local_to_register(&mut self, local: Local, register: Reg) -> io::Result<()> {
        let address = self.local_address(local);
        emit!(self.cx, "mov", "{}, %{}", address, register)
    }

    /// Copia los contenidos de un registro a una local.
    fn register_to_local(&mut self, register: Reg, local: Local) -> io::Result<()> {
        let address = self.local_address(local);
        emit!(self.cx, "mov", "%{}, {}", register, address)
    }

    /// Agrega un offset al puntero de stack.
    fn move_rsp(&mut self, offset: i32) -> io::Result<()> {
        let instruction = if offset < 0 { "subq" } else { "addq" };
        let offset = offset.abs() * VALUE_SIZE as i32;
        emit!(self.cx, instruction, "$0x{:x}, %rsp", offset)
    }

    /// Obtiene el addressing relativo a `%rbp` de una local.
    fn local_address(&self, Local(local): Local) -> String {
        let parameters = self.cx.function().parameters;
        let value_offset = if local < Reg::MAX_ARGS || parameters < Reg::MAX_ARGS {
            -1 - local as i32
        } else if local < parameters {
            1 + (local - Reg::MAX_ARGS) as i32
        } else {
            -1 - (Reg::MAX_ARGS + local - parameters) as i32
        };

        // Los offsets son relativos al frame pointer %rbp
        let offset = value_offset * (VALUE_SIZE as i32);
        let sign = if offset < 0 { "-" } else { "" };
        format!("{}0x{:x}(%rbp)", sign, offset.abs())
    }
}

/// Calcula el padding de stack que se requiere para
/// preservar las condiciones de alineamiento tras una
/// operación de push o equivalente.
fn alignment_for(pushed: u32) -> u32 {
    // Cada valor es de 64 bits (8 bytes), y la frontera de alineamiento es de 16 bytes
    pushed % 2
}
