//! Generación de código.
//!
//! Esta fase consiste en la traducción de representación
//! intermedia (véase [`crate::ir`]) a un lenguaje ensamblador
//! objetivo en particular. Este módulo :

use crate::{
    arch::{Arch, Emitter, Register},
    ir::{GeneratedFunction, Instruction, Label, Local, Program},
};

use std::{
    cell::RefCell,
    fmt,
    io::{self, Write},
};

pub mod regs;

/// Emite código ensamblador para un programa IR.
///
/// Esta función es el punto de entrada del mecanismo de generación
/// de código. Cada función es escrita al flujo de salida según
/// corresponda para la arquitectura objetivo. La salida está destinada
/// a ser utilizada directamente por el GNU assembler y no se esperan
/// otras interpretaciones o manipulaciones antes de ello.
pub fn emit(program: &Program, arch: Arch, output: &mut dyn Write) -> io::Result<()> {
    let value_size = dispatch_arch!(Emitter: arch => Emitter::VALUE_SIZE);

    // Variables globales van en .bss
    for global in &program.globals {
        writeln!(output, ".lcomm {}, {}", global.as_ref(), value_size)?;
    }

    // Inicio de las secciones de código
    writeln!(output, ".text")?;

    // Se emite propiamente cada función no externa
    for function in &program.code {
        dispatch_arch!(Emitter: arch => {
            emit_body::<Emitter>(output, function)?;
        });
    }

    Ok(())
}

/// Contexto de emisión.
///
/// Esta estructura contiene información que las implementaciones
/// de emisión requieren con frecuencia, como lo son el flujo de salida
/// y la función IR que está siendo generada.
pub struct Context<'a, E: Emitter<'a>> {
    function: &'a GeneratedFunction,
    output: RefCell<&'a mut dyn Write>,
    locals: u32,
    next_label: u32,
    frame_info: E::FrameInfo,
}

impl<'a, E: Emitter<'a>> Context<'a, E> {
    /// Función en forma IR que está siendo generada.
    pub fn function(&self) -> &GeneratedFunction {
        self.function
    }

    /// Escribe al flujo de salida.
    pub fn write_fmt(&self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.output.borrow_mut().write_fmt(fmt)
    }

    /// Cantidad máxima de locales que la función accede,
    /// recibe o utiliza en su forma IR.
    ///
    /// Este número se denomina "agnóstico" ya que algunas
    /// implementaciones pueden optar por insertar locales
    /// adicionales por razones que dependen de la arquitectura.
    pub fn agnostic_locals(&self) -> u32 {
        self.locals
    }

    /// Obtiene la información de marco de llamada actual.
    /// Sus contenidos y significado dependen de la arquitectura.
    pub fn frame_info(&self) -> &E::FrameInfo {
        &self.frame_info
    }

    /// Sustituye la información de marco actual para este contexto.
    pub fn with_frame_info(self, frame_info: E::FrameInfo) -> Self {
        Context { frame_info, ..self }
    }

    pub fn next_label(&mut self) -> Label {
        let next_label = self.next_label;
        self.next_label += 1;

        Label(next_label)
    }
}

/// Emite cada una de las instrucciones de una función no externa.
///
/// La correspondencia IR:ensamblador es siempre 1:N.
fn emit_body<'a, E: Emitter<'a>>(
    output: &'a mut dyn Write,
    function: &'a GeneratedFunction,
) -> io::Result<()> {
    let (locals, agnostic_labels) = function.body.iter().map(required_locals_and_labels).fold(
        (0, 0),
        |(max_locals, max_labels), (locals, labels)| {
            (max_locals.max(locals), max_labels.max(labels))
        },
    );

    let locals = locals.max(function.parameters);

    // Colocar cada función en su propia sección permite eliminar
    // código muerto con -Wl,--gc-sections en la fase de enlazado
    writeln!(
        output,
        ".section .text.{0}\n.align {1}\n.global {0}\n{0}:",
        function.name,
        E::VALUE_SIZE
    )?;

    let context = Context {
        function,
        output: RefCell::new(output),
        locals,
        next_label: agnostic_labels,
        frame_info: Default::default(),
    };

    let mut emitter = E::new(context, &function.body)?;
    let mut last_was_unconditional_jump = false;

    for instruction in &function.body {
        use Instruction::*;

        last_was_unconditional_jump = false;

        match instruction {
            Move(from, to) => {
                if *from != *to {
                    let from = emitter.read(*from)?;
                    let to = emitter.write(*to)?;

                    let (cx, _) = emitter.cx_regs();
                    E::reg_to_reg(cx, from, to)?;
                }
            }

            SetLabel(label) => {
                emitter.clear()?;

                let (cx, _) = emitter.cx_regs();
                emit_label!(cx, label)?;
            }

            Jump(label) => {
                let (cx, _) = emitter.cx_regs();
                let label = format_label!(cx, label);

                emitter.spill()?;
                emitter.jump_unconditional(&label)?;

                last_was_unconditional_jump = true;
            }

            JumpIfFalse(local, label) => {
                let (cx, _) = emitter.cx_regs();
                let label = format_label!(cx, label);
                let reg = emitter.read(*local)?;

                emitter.spill()?;
                emitter.jump_if_false(reg, &label)?;
            }

            LoadConst(value, local) => {
                let reg = emitter.write(*local)?;
                emitter.load_const(*value, reg)?;
            }

            LoadGlobal(global, local) => {
                let reg = emitter.write(*local)?;
                emitter.load_global(global, reg)?;
            }

            StoreGlobal(local, global) => {
                let reg = emitter.read(*local)?;
                emitter.store_global(reg, global)?;
            }

            Not(local) => {
                let reg = emitter.read(*local)?;
                emitter.not(reg)?;
                emitter.assert_dirty(reg, *local);
            }

            Negate(local) => {
                let reg = emitter.read(*local)?;
                emitter.negate(reg)?;
                emitter.assert_dirty(reg, *local);
            }

            Binary(lhs, op, rhs) => {
                let lhs_reg = emitter.read(*lhs)?;
                let rhs_reg = emitter.read(*rhs)?;

                emitter.binary(lhs_reg, *op, rhs_reg)?;
                emitter.assert_dirty(lhs_reg, *lhs);
            }

            Call {
                target,
                arguments,
                output,
            } => {
                emitter.spill()?;
                let call_info = emitter.prepare_args(&arguments)?;

                emitter.clear()?;
                emitter.call(&target, call_info)?;

                if let Some(output) = output {
                    emitter.assert_dirty(E::Register::RETURN, *output);
                }
            }
        }
    }

    if !last_was_unconditional_jump {
        emitter.epilogue()?;
    }

    Ok(())
}

/// Cuenta la mínima cantidad de locales y etiquetas que una instrucción exige
/// que se encuentren disponibles y/o en uso.
fn required_locals_and_labels(instruction: &Instruction) -> (u32, u32) {
    use Instruction::*;

    let locals = |Local(local)| local + 1;
    let labels = |Label(label)| label + 1;

    match instruction {
        Move(from, to) => (locals(*from).max(locals(*to)), 0),
        SetLabel(label) => (0, labels(*label)),
        Jump(label) => (0, labels(*label)),
        JumpIfFalse(local, label) => (locals(*local), labels(*label)),
        LoadConst(_, local) => (locals(*local), 0),
        LoadGlobal(_, local) => (locals(*local), 0),
        StoreGlobal(local, _) => (locals(*local), 0),
        Not(local) => (locals(*local), 0),
        Negate(local) => (locals(*local), 0),
        Binary(lhs, _, rhs) => (locals(*lhs).max(locals(*rhs)), 0),

        Call {
            arguments, output, ..
        } => arguments
            .iter()
            .copied()
            .map(locals)
            .max()
            .or(output.map(locals))
            .map(|required| (required, 0))
            .unwrap_or((0, 0)),
    }
}
