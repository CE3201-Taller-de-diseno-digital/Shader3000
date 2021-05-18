//! Manual de ISA: https://0x04.net/~mwk/doc/xtensa.pdf
//! La ABI call0 está documentada en 8.1.2

use super::{emit_label, label_symbol};
use crate::ir::{Function, FunctionBody, Global, Instruction, Local};
use std::{
    fmt,
    io::{self, Write},
    ops::Deref,
};

// Esta es una arquitectura de 32 bits
pub const VALUE_SIZE: u32 = 4;

pub fn emit_function<W: Write>(output: &mut W, function: &Function) -> io::Result<()> {
    let xtensa_function = XtensaFunction {
        output,
        function,
        frame_offset: 0,
    };

    xtensa_function.write_asm()
}

#[derive(Copy, Clone)]
struct Reg(u8);

impl Reg {
    // En call0 se colocan los primeros 6 argumentos en a2-a7
    const MAX_ARGS: u32 = 6;

    const A2: Reg = Reg(2);

    fn argument_sequence() -> impl Iterator<Item = Reg> {
        (2..=7).map(Reg)
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Reg(number) = self;
        write!(formatter, "a{}", number)
    }
}

struct XtensaFunction<'a, W> {
    output: &'a mut W,
    function: &'a Function,
    frame_offset: i32,
}

impl<W: Write> XtensaFunction<'_, W> {
    fn write_asm(mut self) -> io::Result<()> {
        let (inner_locals, instructions) = match &self.function.body {
            FunctionBody::Generated {
                inner_locals,
                instructions,
            } => (inner_locals, instructions),

            _ => return Ok(()),
        };

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
        let total_locals = self.function.parameters + 1 + inner_locals + max_call_spill;
        let padding = if total_locals % 4 == 0 {
            0
        } else {
            4 - total_locals % 4
        };

        // La frontera de alineamiento es de 16 bytes (% 4)
        self.move_sp(-((total_locals + padding) as i32))?;

        // Se preserva la dirección de retorno
        let a0_offset = VALUE_SIZE as i32 * (self.frame_offset - 1);
        emit!(self, "s32i a0, a1, {}", a0_offset)?;

        // Se copian argumentos de registros a locales
        for (register, local) in Reg::argument_sequence().zip(0..self.function.parameters) {
            self.register_to_local(register, Local(local))?;
        }

        // Se emite el cuerpo de la función
        for instruction in instructions {
            self.put_instruction(instruction)?;
        }

        // Epílogo, revierte al estado justo antes de la llamada
        self.move_sp(self.frame_offset)?;
        emit!(self, "l32i a0, a1, -4")?;
        emit!(self, "ret.n")
    }

    fn put_instruction(&mut self, instruction: &Instruction) -> io::Result<()> {
        use Instruction::*;

        match instruction {
            Label(label) => emit_label(self.output, self.function, *label),

            Jump(label) => {
                emit!(self, "j.l {}, a2", label_symbol(self.function, *label))
            }

            JumpIfFalse(local, label) => {
                self.local_to_register(*local, Reg::A2)?;
                emit!(self, "bnez a2, {}", label_symbol(self.function, *label))
            }

            LoadGlobal(global, local) => {
                let Global(global) = global.deref();
                emit!(self, "l32r a2, {}", global)?;
                self.register_to_local(Reg::A2, *local)
            }

            StoreGlobal(local, global) => {
                let Global(global) = global.deref();
                self.local_to_register(*local, Reg::A2)?;

                emit!(self, "movi a3, {}", global)?;
                emit!(self, "s32i a2, a3, 0")
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
        for (i, argument) in arguments.iter().skip(Reg::MAX_ARGS as usize).enumerate() {
            self.local_to_register(*argument, Reg::A2)?;

            let offset = i as u32 * VALUE_SIZE;
            emit!(self, "s32i a2, a1, {}", offset)?;
        }

        // Los primeros seis argumentos se colocan en registros específicos
        for (argument, register) in arguments.iter().zip(Reg::argument_sequence()) {
            self.local_to_register(*argument, register)?;
        }

        emit!(self, "call0 {}", target.name)?;
        if let Some(output_local) = output_local {
            self.register_to_local(Reg::A2, output_local)?;
        }

        Ok(())
    }

    fn local_to_register(&mut self, local: Local, register: Reg) -> io::Result<()> {
        self.load_or_store(register, local, "l32i")
    }

    fn register_to_local(&mut self, register: Reg, local: Local) -> io::Result<()> {
        self.load_or_store(register, local, "s32i")
    }

    fn load_or_store(&mut self, register: Reg, local: Local, instruction: &str) -> io::Result<()> {
        let address = self.local_address(local);
        emit!(self, "{} {}, {}", instruction, register, address)
    }

    fn move_sp(&mut self, offset: i32) -> io::Result<()> {
        self.frame_offset -= offset;
        emit!(self, "addi a1, a1, {}", offset * VALUE_SIZE as i32)
    }

    fn local_address(&self, Local(local): Local) -> String {
        let parameters = self.function.parameters;
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
