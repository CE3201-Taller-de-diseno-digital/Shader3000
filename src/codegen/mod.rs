use crate::ir::{Function, FunctionBody, Global, Label, Program};
use std::{
    io::{self, Write},
    ops::Deref,
};

pub enum Architecture {
    X86_64,
    Xtensa,
}

pub fn write<W: Write>(program: &Program, arch: Architecture, output: &mut W) -> io::Result<()> {
    use Architecture::*;

    let value_size = match arch {
        X86_64 => todo!(),
        Xtensa => todo!(),
    };

    for global in &program.globals {
        let Global(name) = global.deref();
        writeln!(output, ".lcomm {}, {}", name, value_size)?;
    }

    writeln!(output, ".text")?;

    for function in &program.code {
        let name = match function.body {
            FunctionBody::Generated { .. } => &function.name,
            _ => continue,
        };

        writeln!(output, ".global {0}\n{0}:", name)?;

        match arch {
            X86_64 => todo!(),
            Xtensa => todo!(),
        }
    }

    Ok(())
}
