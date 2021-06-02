//! Implementación para Tensilica Xtensa.
//!
//! # Manual de ISA
//! <https://0x04.net/~mwk/doc/xtensa.pdf>
//!
//! La ABI `call0` está documentada en 8.1.2.

use crate::{
    codegen::Context,
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
#[derive(Copy, Clone)]
pub struct Reg(u8);

impl Reg {
    /// Para la ABI `call0` se colocan los primeros seis argumentos en `a2`-`a7`.
    const MAX_ARGS: u32 = 6;

    /// Registro `a2`.
    const A2: Reg = Reg(2);

    /// Secuencia de registros en los que se colocan los primeros argumentos.
    fn argument_sequence() -> impl Iterator<Item = Reg> {
        (2..=7).map(Reg)
    }
}

impl super::Register for Reg {}

impl fmt::Display for Reg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Reg(number) = self;
        write!(formatter, "a{}", number)
    }
}

/// Implementación de emisión de código para Xtensa.
pub struct Emitter<'a> {
    cx: Context<'a, Self>,
}

/// Información de estado para cada frame.
#[derive(Default)]
pub struct FrameInfo {
    offset: i32,
}

impl<'a> super::Emitter<'a> for Emitter<'a> {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Register = Reg;
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
        };

        emitter.move_sp(-frame_offset)?;

        // Se preserva la dirección de retorno
        let a0_offset = VALUE_SIZE as i32 * (frame_offset - 1);
        emit!(emitter.cx, "s32i", "a0, a1, {}", a0_offset)?;

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

    fn epilogue(self) -> io::Result<()> {
        // Revierte al estado justo antes de la llamada
        self.move_sp(self.cx.frame_info().offset)?;
        emit!(self.cx, "l32i", "a0, a1, -4")?;
        emit!(self.cx, "ret.n")
    }

    fn jump_unconditional(&mut self, label: &str) -> io::Result<()> {
        emit!(self.cx, "j.l", "{}, a2", label)
    }

    fn jump_if_false(&mut self, local: Local, label: &str) -> io::Result<()> {
        self.local_to_register(local, Reg::A2)?;
        emit!(self.cx, "bnez", "a2, {}", label)
    }

    fn load_const(&mut self, value: i32, local: Local) -> io::Result<()> {
        emit!(self.cx, "movi", "a2, {}", value)?;
        self.register_to_local(Reg::A2, local)
    }

    fn load_global(&mut self, Global(global): &Global, local: Local) -> io::Result<()> {
        emit!(self.cx, "movi", "a2, {}", global)?;
        emit!(self.cx, "l32i", "a2, a2, 0")?;
        self.register_to_local(Reg::A2, local)
    }

    fn store_global(&mut self, local: Local, Global(global): &Global) -> io::Result<()> {
        self.local_to_register(local, Reg::A2)?;
        emit!(self.cx, "movi", "a3, {}", global)?;
        emit!(self.cx, "s32i", "a2, a3, 0")
    }

    fn call(
        &mut self,
        target: &Function,
        arguments: &[Local],
        output_local: Option<Local>,
    ) -> io::Result<()> {
        // Argumentos del séptimo en adelante se colocan en stack en orden inverso
        for (i, argument) in arguments.iter().skip(Reg::MAX_ARGS as usize).enumerate() {
            self.local_to_register(*argument, Reg::A2)?;

            let offset = i as u32 * VALUE_SIZE;
            emit!(self.cx, "s32i", "a2, a1, {}", offset)?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, register) in arguments.iter().zip(Reg::argument_sequence()) {
            self.local_to_register(*argument, register)?;
        }

        emit!(self.cx, "call0", "{}", target.name)?;
        if let Some(output_local) = output_local {
            self.register_to_local(Reg::A2, output_local)?;
        }

        Ok(())
    }
}

impl Emitter<'_> {
    /// Copia los contenidos de una local a un registro.
    fn local_to_register(&mut self, local: Local, register: Reg) -> io::Result<()> {
        self.load_or_store(register, local, "l32i")
    }

    /// Copia los contenidos de un registro a una local.
    fn register_to_local(&mut self, register: Reg, local: Local) -> io::Result<()> {
        self.load_or_store(register, local, "s32i")
    }

    /// Copia entre una local y un registro.
    fn load_or_store(&mut self, register: Reg, local: Local, instruction: &str) -> io::Result<()> {
        let address = self.local_address(local);
        emit!(self.cx, instruction, "{}, {}", register, address)
    }

    /// Corrige el registro de puntero de stack.
    fn move_sp(&self, offset: i32) -> io::Result<()> {
        emit!(self.cx, "addi", "a1, a1, {}", offset * VALUE_SIZE as i32)
    }

    /// Determina la posición de una
    fn local_address(&self, Local(local): Local) -> String {
        let parameters = self.cx.function().parameters;
        let value_offset = if local < Reg::MAX_ARGS || parameters < Reg::MAX_ARGS {
            -2 - local as i32
        } else if local < parameters {
            local as i32
        } else {
            -2 - (Reg::MAX_ARGS + local - parameters) as i32
        };

        let offset = (self.cx.frame_info().offset + value_offset) * (VALUE_SIZE as i32);
        format!("a1, {}", offset.abs())
    }
}
