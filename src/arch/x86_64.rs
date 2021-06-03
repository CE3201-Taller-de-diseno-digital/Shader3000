//! Implementación para x86-64.

use crate::{
    codegen::{regs::Allocations, Context},
    ir::{Function, Global, Instruction, Local},
};

use std::{fmt, io};

/// Esta es una arquitectura de 64 bits
const VALUE_SIZE: u32 = 8;

/// Registro de procesador.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Reg {
    Rax,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
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

    /// Obtiene la forma de 32 bits de un registro x86.
    fn as_dword(self) -> &'static str {
        use Reg::*;

        match self {
            Rax => "eax",
            Rcx => "ecx",
            Rdx => "edx",
            Rsi => "esi",
            Rdi => "edi",
            R8 => "r8d",
            R9 => "r9d",
            R10 => "r10d",
            R11 => "r11d",
        }
    }
}

impl super::Register for Reg {
    const RETURN: Self = Reg::Rax;
    const FILE: &'static [Self] = &[
        Reg::Rax,
        Reg::Rdi,
        Reg::Rsi,
        Reg::Rdx,
        Reg::Rcx,
        Reg::R8,
        Reg::R9,
        Reg::R10,
        Reg::R11,
    ];
}

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
            R10 => "r10",
            R11 => "r11",
        };

        formatter.write_str(name)
    }
}

/// Emisión de código para x86-64.
pub struct Emitter<'a> {
    cx: Context<'a, Self>,
    regs: Allocations<'a, Self>,
}

/// Información que debe preservarse durante una llamada.
pub struct CallInfo {
    rsp_offset: u32,
}

impl<'a> super::Emitter<'a> for Emitter<'a> {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Register = Reg;
    type CallInfo = CallInfo;
    type FrameInfo = ();

    fn new(cx: Context<'a, Self>, _: &[Instruction]) -> io::Result<Self> {
        // Prólogo, se crea un stack frame
        emit!(cx, "push", "%rbp")?;
        emit!(cx, "mov", "%rsp, %rbp")?;

        // Se reserva memoria para locales
        let total_locals = cx.agnostic_locals();
        let stack_allocation = total_locals + alignment_for(total_locals);

        let mut emitter = Emitter {
            cx,
            regs: Default::default(),
        };

        if stack_allocation > 0 {
            emitter.move_rsp(-(stack_allocation as i32))?;
        }

        // Se definen posiciones de argumentos en registros
        let parameters = emitter.cx.function().parameters;
        for (reg, local) in Reg::argument_sequence().zip((0..parameters).map(Local)) {
            emitter.assert_dirty(reg, local);
        }

        Ok(emitter)
    }

    fn cx_regs(&mut self) -> (&mut Context<'a, Self>, &mut Allocations<'a, Self>) {
        (&mut self.cx, &mut self.regs)
    }

    fn epilogue(self) -> io::Result<()> {
        // Revierte al estado justo antes de la llamada
        emit!(self.cx, "mov", "%rbp, %rsp")?;
        emit!(self.cx, "pop", "%rbp")?;
        emit!(self.cx, "ret")
    }

    fn jump_unconditional(&mut self, label: &str) -> io::Result<()> {
        emit!(self.cx, "jmp", "{}", label)
    }

    fn jump_if_false(&mut self, reg: Reg, label: &str) -> io::Result<()> {
        emit!(self.cx, "testl", "%{0}, %{0}", reg.as_dword())?;
        emit!(self.cx, "jz", "{}", label)
    }

    fn load_const(&mut self, value: i32, reg: Reg) -> io::Result<()> {
        if value == 0 {
            emit!(self.cx, "xor", "%{0}, %{0}", reg.as_dword())
        } else if value > 0 {
            emit!(self.cx, "mov", "${}, %{}", value, reg.as_dword())
        } else {
            emit!(self.cx, "mov", "${}, %{}", value, reg)
        }
    }

    fn load_global(&mut self, Global(global): &Global, reg: Reg) -> io::Result<()> {
        emit!(self.cx, "mov", "{}(%rip), %{}", global, reg)
    }

    fn store_global(&mut self, reg: Reg, Global(global): &Global) -> io::Result<()> {
        emit!(self.cx, "mov", "%{}, {}(%rip)", reg, global)
    }

    fn prepare_args(&mut self, arguments: &[Local]) -> io::Result<CallInfo> {
        // Argumentos del séptimo en adelante se colocan en stack en orden inverso
        let pushed = (arguments.len() as u32).max(Reg::MAX_ARGS) - Reg::MAX_ARGS;

        // Corrección del stack pointer alrededor de la llamada, manteniendo el alineamiento de 16 bytes
        let rsp_offset = if arguments.len() as u32 > Reg::MAX_ARGS {
            let alignment = alignment_for(pushed);
            if alignment > 0 {
                self.move_rsp(-(alignment as i32))?;
            }

            pushed + alignment
        } else {
            0
        };

        for argument in arguments.iter().rev().take(pushed as usize) {
            let address = Self::local_address(&self.cx, *argument);
            emit!(self.cx, "push", "{}", address)?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, reg) in arguments.iter().zip(Reg::argument_sequence()) {
            self.cx.read_into(&mut self.regs, reg, *argument)?;
        }

        Ok(CallInfo { rsp_offset })
    }

    fn call(&mut self, target: &Function, call_info: CallInfo) -> io::Result<()> {
        emit!(self.cx, "call", "{}", target.name)?;

        // Se reclama memoria que fue usada para argumentos
        if call_info.rsp_offset > 0 {
            self.move_rsp(call_info.rsp_offset as i32)?;
        }

        Ok(())
    }

    fn reg_to_local(cx: &Context<'a, Self>, reg: Reg, local: Local) -> io::Result<()> {
        let address = Self::local_address(cx, local);
        emit!(cx, "mov", "%{}, {}", reg, address)
    }

    fn local_to_reg(cx: &Context<'a, Self>, local: Local, reg: Reg) -> io::Result<()> {
        let address = Self::local_address(cx, local);
        emit!(cx, "mov", "{}, %{}", address, reg)
    }

    fn reg_to_reg(cx: &Context<'a, Self>, source: Reg, target: Reg) -> io::Result<()> {
        emit!(cx, "mov", "%{}, %{}", source, target)
    }
}

impl<'a> Emitter<'a> {
    /// Agrega un offset al puntero de stack.
    fn move_rsp(&mut self, offset: i32) -> io::Result<()> {
        let instruction = if offset < 0 { "subq" } else { "addq" };
        let offset = offset.abs() * VALUE_SIZE as i32;
        emit!(self.cx, instruction, "$0x{:x}, %rsp", offset)
    }

    /// Obtiene el addressing relativo a `%rbp` de una local.
    fn local_address(cx: &Context<'a, Self>, Local(local): Local) -> String {
        let parameters = cx.function().parameters;
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
