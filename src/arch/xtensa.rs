//! Implementación para Tensilica Xtensa.
//!
//! # Manual de ISA
//! <https://0x04.net/~mwk/doc/xtensa.pdf>
//!
//! La ABI `call0` está documentada en 8.1.2.

use crate::{
    codegen::{regs::Allocations, Context},
    ir::{Function, Global, Instruction, Local},
};

use std::{fmt, io};

/// Esta es una arquitectura de 32 bits.
const VALUE_SIZE: u32 = 4;

/// Registro de procesador.
///
/// La arquitectura expone 16 registros de propósito general,
/// `a0` hasta `a15`, en todo momento. Existe una ventana de
/// registros que extiende esto hasta cuatro veces. Esta
/// implementación no hace uso de la ventana de registros.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Reg(u8);

impl Reg {
    /// Para la ABI `call0` se colocan los primeros seis argumentos en `a2`-`a7`.
    const MAX_ARGS: u32 = 6;

    /// Secuencia de registros en los que se colocan los primeros argumentos.
    fn argument_sequence() -> impl Iterator<Item = Reg> {
        (2..=7).map(Reg)
    }
}

impl super::Register for Reg {
    const RETURN: Self = Reg(2);
    const FILE: &'static [Self] = &[Reg(2), Reg(3), Reg(4), Reg(5), Reg(6), Reg(7), Reg(8)];
}

impl fmt::Display for Reg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Reg(number) = self;
        write!(formatter, "a{}", number)
    }
}

/// Implementación de emisión de código para Xtensa.
pub struct Emitter<'a> {
    cx: Context<'a, Self>,
    regs: Allocations<'a, Self>,
}

/// Información de estado para cada frame.
#[derive(Default)]
pub struct FrameInfo {
    offset: i32,
}

impl<'a> super::Emitter<'a> for Emitter<'a> {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Register = Reg;
    type CallInfo = ();
    type FrameInfo = FrameInfo;

    fn new(cx: Context<'a, Self>, instructions: &[Instruction]) -> io::Result<Self> {
        // Xtensa no tiene push/pop, por lo cual esto evita mucho trabajo sobre a1/sp
        let max_call_spill = instructions
            .iter()
            .map(|instruction| match instruction {
                Instruction::Call { arguments, .. } => {
                    (arguments.len() as u32).max(Reg::MAX_ARGS) - Reg::MAX_ARGS
                }

                _ => 0,
            })
            .max()
            .unwrap_or(0);

        // Se reserva memoria para locales. "+ 1" debido a que se debe preservar a0
        let total_locals = cx.agnostic_locals() + 1 + max_call_spill;

        // Alineamiento de 16 bytes (4 * 4 bytes)
        let padding = if total_locals % 4 == 0 {
            0
        } else {
            4 - total_locals % 4
        };

        // La frontera de alineamiento es de 16 bytes (% 4)
        let frame_offset = (total_locals + padding) as i32;
        let cx = cx.with_frame_info(FrameInfo {
            offset: frame_offset,
        });

        let mut emitter = Emitter {
            cx,
            regs: Default::default(),
        };

        emitter.move_sp(-frame_offset)?;

        // Se preserva la dirección de retorno
        let a0_offset = VALUE_SIZE as i32 * (frame_offset - 1);
        emit!(emitter.cx, "s32i", "a0, a1, {}", a0_offset)?;

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
        self.move_sp(self.cx.frame_info().offset)?;
        emit!(self.cx, "l32i", "a0, a1, -4")?;
        emit!(self.cx, "ret.n")
    }

    fn jump_unconditional(&mut self, label: &str) -> io::Result<()> {
        let reg = self.cx.scratch(&mut self.regs, &[])?;
        emit!(self.cx, "j.l", "{}, {}", label, reg)
    }

    fn jump_if_false(&mut self, reg: Reg, label: &str) -> io::Result<()> {
        emit!(self.cx, "bnez", "{}, {}", reg, label)
    }

    fn load_const(&mut self, value: i32, reg: Reg) -> io::Result<()> {
        emit!(self.cx, "movi", "{}, {}", reg, value)
    }

    fn load_global(&mut self, Global(global): &Global, reg: Reg) -> io::Result<()> {
        emit!(self.cx, "movi", "{}, {}", reg, global)?;
        emit!(self.cx, "l32i", "{0}, {0}, 0", reg)
    }

    fn store_global(&mut self, reg: Reg, Global(global): &Global) -> io::Result<()> {
        let scratch = self.cx.scratch(&mut self.regs, &[reg])?;
        emit!(self.cx, "movi", "{}, {}", scratch, global)?;
        emit!(self.cx, "s32i", "{}, {}, 0", reg, scratch)
    }

    fn prepare_args(&mut self, arguments: &[Local]) -> io::Result<()> {
        // Argumentos del séptimo en adelante se colocan en stack en orden inverso
        for (i, argument) in arguments.iter().skip(Reg::MAX_ARGS as usize).enumerate() {
            let reg = self.read(*argument)?;
            let offset = i as u32 * VALUE_SIZE;

            emit!(self.cx, "s32i", "{}, a1, {}", reg, offset)?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, reg) in arguments.iter().zip(Reg::argument_sequence()) {
            self.cx.read_into(&mut self.regs, reg, *argument)?;
        }

        Ok(())
    }

    fn call(&mut self, target: &Function, _call_info: ()) -> io::Result<()> {
        emit!(self.cx, "call0", "{}", target.name)
    }

    fn reg_to_local(cx: &Context<'a, Self>, reg: Reg, local: Local) -> io::Result<()> {
        Self::load_or_store(cx, reg, local, "s32i")
    }

    fn local_to_reg(cx: &Context<'a, Self>, local: Local, reg: Reg) -> io::Result<()> {
        Self::load_or_store(cx, reg, local, "l32i")
    }

    fn reg_to_reg(cx: &Context<'a, Self>, source: Reg, target: Reg) -> io::Result<()> {
        emit!(cx, "mov.n", "{}, {}", target, source)
    }
}

impl<'a> Emitter<'a> {
    /// Corrige el registro de puntero de stack.
    fn move_sp(&self, offset: i32) -> io::Result<()> {
        emit!(self.cx, "addi", "a1, a1, {}", offset * VALUE_SIZE as i32)
    }

    /// Copia entre una local y un registro.
    fn load_or_store(
        cx: &Context<'a, Self>,
        reg: Reg,
        local: Local,
        instruction: &str,
    ) -> io::Result<()> {
        let address = Self::local_address(cx, local);
        emit!(cx, instruction, "{}, {}", reg, address)
    }

    /// Determina la posición de una
    fn local_address(cx: &Context<'a, Self>, Local(local): Local) -> String {
        let parameters = cx.function().parameters;
        let value_offset = if local < Reg::MAX_ARGS || parameters < Reg::MAX_ARGS {
            -2 - local as i32
        } else if local < parameters {
            local as i32
        } else {
            -2 - (Reg::MAX_ARGS + local - parameters) as i32
        };

        let offset = (cx.frame_info().offset + value_offset) * (VALUE_SIZE as i32);
        format!("a1, {}", offset.abs())
    }
}
