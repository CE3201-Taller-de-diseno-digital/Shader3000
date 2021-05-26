use crate::{
    arch::Arch,
    ir::{Function, FunctionBody, Global, Label, Program},
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
        if let FunctionBody::Generated { .. } = function.body {
            dispatch_arch!(Target: arch => Target::emit_function(output, function))?;
        }
    }

    Ok(())
}

pub fn emit_label<W: Write>(
    output: &mut W,
    function: &Function,
    Label(label): Label,
) -> io::Result<()> {
    writeln!(output, "\t.L{}.{}:", function.name, label)
}

pub fn label_symbol(function: &Function, Label(label): Label) -> String {
    format!(".L{}.{}", function.name, label)
}
