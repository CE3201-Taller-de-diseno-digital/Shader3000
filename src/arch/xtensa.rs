//! Manual de ISA: <https://0x04.net/~mwk/doc/xtensa.pdf>.
//! La ABI call0 está documentada en 8.1.2

use crate::{
    codegen::Context,
    ir::{Function, Global, Instruction, Local},
};
use std::{
    fmt,
    io::{self, Write},
};

// Esta es una arquitectura de 32 bits
const VALUE_SIZE: u32 = 4;

pub struct Target;

impl super::Target for Target {
    const VALUE_SIZE: u32 = VALUE_SIZE;

    type Emitter = Emitter;
    type Register = Reg;
}

#[derive(Copy, Clone)]
pub struct Reg(u8);

impl Reg {
    // En call0 se colocan los primeros 6 argumentos en a2-a7
    const MAX_ARGS: u32 = 6;

    const A2: Reg = Reg(2);

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

pub struct Emitter {
    frame_offset: i32,
    max_call_spill: u32,
}

impl super::Emitter for Emitter {
    fn new(instructions: &[Instruction]) -> Self {
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

        Emitter {
            max_call_spill,
            frame_offset: 0,
        }
    }

    fn prologue<W: Write>(&mut self, cx: &mut Context<W>) -> io::Result<()> {
        // Se reserva memoria para locales. "+ 1" debido a que se debe preservar a0
        let total_locals = cx.agnostic_locals() + 1 + self.max_call_spill;
        let padding = if total_locals % 4 == 0 {
            0
        } else {
            4 - total_locals % 4
        };

        // La frontera de alineamiento es de 16 bytes (% 4)
        self.move_sp(cx, -((total_locals + padding) as i32))?;

        // Se preserva la dirección de retorno
        let a0_offset = VALUE_SIZE as i32 * (self.frame_offset - 1);
        emit!(cx, "s32i", "a0, a1, {}", a0_offset)?;

        // Se copian argumentos de registros a locales
        for (register, local) in Reg::argument_sequence().zip(0..cx.function().parameters) {
            self.register_to_local(cx, register, Local(local))?;
        }

        Ok(())
    }

    fn epilogue<W: Write>(&mut self, cx: &mut Context<W>) -> io::Result<()> {
        // Revierte al estado justo antes de la llamada
        self.move_sp(cx, self.frame_offset)?;
        emit!(cx, "l32i", "a0, a1, -4")?;
        emit!(cx, "ret.n")
    }

    fn jump_unconditional<W: Write>(&mut self, cx: &mut Context<W>, label: &str) -> io::Result<()> {
        emit!(cx, "j.l", "{}, a2", label)
    }

    fn jump_if_false<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        label: &str,
    ) -> io::Result<()> {
        self.local_to_register(cx, local, Reg::A2)?;
        emit!(cx, "bnez", "a2, {}", label)
    }

    fn load_global<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        Global(global): &Global,
        local: Local,
    ) -> io::Result<()> {
        emit!(cx, "movi", "a2, {}", global)?;
        emit!(cx, "l32i", "a2, a2, 0")?;
        self.register_to_local(cx, Reg::A2, local)
    }

    fn store_global<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        local: Local,
        Global(global): &Global,
    ) -> io::Result<()> {
        self.local_to_register(cx, local, Reg::A2)?;
        emit!(cx, "movi", "a3, {}", global)?;
        emit!(cx, "s32i", "a2, a3, 0")
    }

    fn call<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        target: &Function,
        arguments: &[Local],
        output_local: Option<Local>,
    ) -> io::Result<()> {
        // Argumentos del séptimo en adelante se colocan en stack en orden inverso
        for (i, argument) in arguments.iter().skip(Reg::MAX_ARGS as usize).enumerate() {
            self.local_to_register(cx, *argument, Reg::A2)?;

            let offset = i as u32 * VALUE_SIZE;
            emit!(cx, "s32i", "a2, a1, {}", offset)?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, register) in arguments.iter().zip(Reg::argument_sequence()) {
            self.local_to_register(cx, *argument, register)?;
        }

        emit!(cx, "call0", "{}", target.name)?;
        if let Some(output_local) = output_local {
            self.register_to_local(cx, Reg::A2, output_local)?;
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
        self.load_or_store(cx, register, local, "l32i")
    }

    fn register_to_local<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        register: Reg,
        local: Local,
    ) -> io::Result<()> {
        self.load_or_store(cx, register, local, "s32i")
    }

    fn load_or_store<W: Write>(
        &mut self,
        cx: &mut Context<W>,
        register: Reg,
        local: Local,
        instruction: &str,
    ) -> io::Result<()> {
        let address = self.local_address(cx, local);
        emit!(cx, instruction, "{}, {}", register, address)
    }

    fn move_sp<W: Write>(&mut self, cx: &mut Context<W>, offset: i32) -> io::Result<()> {
        self.frame_offset -= offset;
        emit!(cx, "addi", "a1, a1, {}", offset * VALUE_SIZE as i32)
    }

    fn local_address<W: Write>(&self, cx: &Context<W>, Local(local): Local) -> String {
        let parameters = cx.function().parameters;
        let value_offset = if local < Reg::MAX_ARGS || parameters < Reg::MAX_ARGS {
            -2 - local as i32
        } else if local < parameters {
            local as i32
        } else {
            -2 - (Reg::MAX_ARGS + local - parameters) as i32
        };

        let offset = (self.frame_offset + value_offset) * (VALUE_SIZE as i32);
        format!("a1, {}", offset.abs())
    }
}
