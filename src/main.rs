use anyhow::{self, Context};
use clap::{self, crate_version, Arg};
use compiler::{
    ir::*,
    target::{self, Arch},
};

use std::{fs::File, rc::Rc, str::FromStr};

fn main() -> anyhow::Result<()> {
    let args = clap::App::new("AnimationLed compiler")
        .version(crate_version!())
        .arg(
            Arg::new("target")
                .short('t')
                .value_name("ARCH")
                .takes_value(true)
                .default_value("x86_64")
                .possible_values(&["x86_64", "xtensa"])
                .about("Target architecture"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .takes_value(true)
                .required(true)
                .about("Output file ('-' for stdout)"),
        )
        .get_matches();

    let program = test_program();
    let arch =
        Arch::from_str(&args.value_of("target").unwrap()).expect("main.rs allowed a bad target");

    let result = match args.value_of("output").unwrap() {
        "-" => {
            let mut stdout = std::io::stdout();
            target::emit(&program, arch, &mut stdout)
        }

        path => {
            let mut file = File::create(path)
                .with_context(|| format!("Failed to open for writing: {}", path))?;

            target::emit(&program, arch, &mut file)
        }
    };

    result.context("Failed to emit generated code").into()
}

fn test_program() -> Program {
    let builtin_zero = Rc::new(Function {
        name: String::from("builtin_zero"),
        parameters: 0,
        body: FunctionBody::External,
    });

    let builtin_inc = Rc::new(Function {
        name: String::from("builtin_inc"),
        parameters: 1,
        body: FunctionBody::External,
    });

    let builtin_delay_seg = Rc::new(Function {
        name: String::from("builtin_delay_seg"),
        parameters: 1,
        body: FunctionBody::External,
    });

    let builtin_debug = Rc::new(Function {
        name: String::from("builtin_debug"),
        parameters: 1,
        body: FunctionBody::External,
    });

    let user_main = Rc::new(Function {
        name: String::from("user_main"),
        parameters: 0,
        body: FunctionBody::Generated(vec![
            Instruction::Call {
                target: Rc::clone(&builtin_zero),
                arguments: vec![],
                output: Some(Local(0)),
            },
            Instruction::SetLabel(Label(0)),
            Instruction::Call {
                target: Rc::clone(&builtin_inc),
                arguments: vec![Local(0)],
                output: Some(Local(0)),
            },
            Instruction::Call {
                target: Rc::clone(&builtin_debug),
                arguments: vec![Local(0)],
                output: None,
            },
            Instruction::Call {
                target: Rc::clone(&builtin_delay_seg),
                arguments: vec![Local(0)],
                output: None,
            },
            Instruction::Jump(Label(0)),
        ]),
    });

    Program {
        globals: vec![],
        code: vec![user_main],
    }
}
