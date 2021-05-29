use crate::{
    arch::{Arch, Emitter},
    ir::{Function, FunctionBody, Global, Instruction, Label, Local, Program},
};

use std::{
    io::{self, Write},
    ops::Deref,
};

pub fn emit(program: &Program, arch: Arch, output: &mut dyn Write) -> io::Result<()> {
    let value_size = dispatch_arch!(Emitter: arch => Emitter::VALUE_SIZE);

    for global in &program.globals {
        let Global(name) = global.deref();
        writeln!(output, ".lcomm {}, {}", name, value_size)?;
    }

    writeln!(output, ".text\n.global user_main")?;

    for function in &program.code {
        if let FunctionBody::Generated(instructions) = &function.body {
            dispatch_arch!(Emitter: arch => {
                emit_body::<Emitter>(output, function, &instructions)?;
            });
        }
    }

    Ok(())
}

pub struct Context<'a, E: Emitter<'a>> {
    function: &'a Function,
    output: &'a mut dyn Write,
    locals: u32,
    _todo: std::marker::PhantomData<[E; 0]>,
}

impl<'a, E: Emitter<'a>> Context<'a, E> {
    pub fn function(&self) -> &Function {
        self.function
    }

    pub fn output(&mut self) -> &mut dyn Write {
        self.output
    }

    pub fn agnostic_locals(&self) -> u32 {
        self.locals
    }
}

fn emit_body<'a, E: Emitter<'a>>(
    output: &'a mut dyn Write,
    function: &'a Function,
    instructions: &[Instruction],
) -> io::Result<()> {
    let locals = instructions
        .iter()
        .map(required_locals)
        .max()
        .unwrap_or(0)
        .max(function.parameters);

    writeln!(output, ".section .text.{0}\n{0}:", function.name)?;

    let context = Context {
        function,
        output,
        locals,
        _todo: Default::default(),
    };

    let mut emitter = E::new(context, instructions)?;
    for instruction in instructions {
        use Instruction::*;

        match instruction {
            SetLabel(Label(label)) => {
                writeln!(emitter.cx().output, "\t.L{}.{}:", function.name, label)?;
            }

            Jump(label) => {
                let label = label_symbol(function, *label);
                emitter.jump_unconditional(&label)?;
            }

            JumpIfFalse(local, label) => {
                let label = label_symbol(function, *label);
                emitter.jump_if_false(*local, &label)?;
            }

            LoadConst(value, local) => emitter.load_const(*value, *local)?,
            LoadGlobal(global, local) => emitter.load_global(global, *local)?,
            StoreGlobal(local, global) => emitter.store_global(*local, global)?,

            Call {
                target,
                arguments,
                output,
            } => {
                emitter.call(&target, &arguments, *output)?;
            }
        }
    }

    emitter.epilogue()
}

fn label_symbol(function: &Function, Label(label): Label) -> String {
    format!(".L{}.{}", function.name, label)
}

fn required_locals(instruction: &Instruction) -> u32 {
    use Instruction::*;

    let required = |Local(local)| local + 1;
    match instruction {
        JumpIfFalse(local, _) => required(*local),
        LoadConst(_, local) => required(*local),
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
