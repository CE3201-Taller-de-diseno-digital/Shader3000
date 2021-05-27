use crate::{
    arch::{Arch, Emitter, Target},
    ir::{Function, FunctionBody, Global, Instruction, Label, Local, Program},
};

use std::{
    io::{self, Write},
    ops::Deref,
};

pub fn emit_asm<W: Write>(program: &Program, arch: Arch, output: &mut W) -> io::Result<()> {
    let value_size = dispatch_arch!(Target: arch => Target::VALUE_SIZE);

    for global in &program.globals {
        let Global(name) = global.deref();
        writeln!(output, ".lcomm {}, {}", name, value_size)?;
    }

    writeln!(output, ".text\n.global user_main")?;

    for function in &program.code {
        if let FunctionBody::Generated(instructions) = &function.body {
            dispatch_arch!(Target: arch => {
                emit_body::<Target, W>(output, function, &instructions)?;
            });
        }
    }

    Ok(())
}

pub struct Context<'a, W> {
    function: &'a Function,
    output: &'a mut W,
    locals: u32,
}

impl<W> Context<'_, W> {
    pub fn function(&self) -> &Function {
        self.function
    }

    pub fn output(&mut self) -> &mut W {
        self.output
    }

    pub fn agnostic_locals(&self) -> u32 {
        self.locals
    }
}

fn emit_body<T: Target, W: Write>(
    output: &mut W,
    function: &Function,
    instructions: &[Instruction],
) -> io::Result<()> {
    let locals = instructions
        .iter()
        .map(required_locals)
        .max()
        .unwrap_or(0)
        .max(function.parameters);

    writeln!(output, ".section .text.{0}\n{0}:", function.name)?;
    let mut context = Context {
        function,
        output,
        locals,
    };

    let mut emitter = T::Emitter::new(instructions);
    emitter.prologue(&mut context)?;

    for instruction in instructions {
        use Instruction::*;

        match instruction {
            SetLabel(Label(label)) => {
                writeln!(context.output, "\t.L{}.{}:", function.name, label)?;
            }

            Jump(label) => {
                let label = label_symbol(function, *label);
                emitter.jump_unconditional(&mut context, &label)?;
            }

            JumpIfFalse(local, label) => {
                let label = label_symbol(function, *label);
                emitter.jump_if_false(&mut context, *local, &label)?;
            }

            LoadGlobal(global, local) => emitter.load_global(&mut context, global, *local)?,
            StoreGlobal(local, global) => emitter.store_global(&mut context, *local, global)?,

            Call {
                target,
                arguments,
                output,
            } => {
                emitter.call(&mut context, &target, &arguments, *output)?;
            }
        }
    }

    emitter.epilogue(&mut context)
}

fn label_symbol(function: &Function, Label(label): Label) -> String {
    format!(".L{}.{}", function.name, label)
}

fn required_locals(instruction: &Instruction) -> u32 {
    use Instruction::*;

    let required = |Local(local)| local + 1;
    match instruction {
        JumpIfFalse(local, _) => required(*local),
        LoadGlobal(_, local) => required(*local),
        StoreGlobal(local, _) => required(*local),

        Call {
            arguments, output, ..
        } => arguments
            .iter()
            .copied()
            .map(required)
            .max()
            .or(output.map(required))
            .unwrap_or(0),

        SetLabel(_) | Jump(_) => 0,
    }
}
