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
        X86_64 => x86_64::VALUE_SIZE,
        Xtensa => xtensa::VALUE_SIZE,
    };

    for global in &program.globals {
        let Global(name) = global.deref();
        writeln!(output, ".lcomm {}, {}", name, value_size)?;
    }

    writeln!(output, ".text\n.global user_main")?;

    for function in &program.code {
        if let FunctionBody::Generated { .. } = function.body {
            match arch {
                X86_64 => x86_64::emit_function(output, function)?,
                Xtensa => xtensa::emit_function(output, function)?,
            }
        }
    }

    Ok(())
}

macro_rules! emit {
    ($self:expr, $($format:tt)*) => {{
        write!($self.output, "\t")?;
        writeln!($self.output, $($format)*)
    }};
}

mod x86_64;
mod xtensa;

fn emit_label<W: Write>(
    output: &mut W,
    function: &Function,
    Label(label): Label,
) -> io::Result<()> {
    writeln!(output, "\t.L{}.{}:", function.name, label)
}

fn label_symbol(function: &Function, Label(label): Label) -> String {
    format!(".L{}.{}", function.name, label)
}
