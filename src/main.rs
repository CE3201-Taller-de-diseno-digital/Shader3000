use anyhow::{self, Context};
use clap::{self, crate_version, Arg};
use compiler::{
    codegen::{self, Architecture},
    ir::*,
};

use std::{fs::File, rc::Rc};

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
    let arch = match args.value_of("target").unwrap() {
        "x86_64" => Architecture::X86_64,
        "xtensa" => Architecture::Xtensa,
        _ => unreachable!(),
    };

    let result = match args.value_of("output").unwrap() {
        "-" => {
            let mut stdout = std::io::stdout();
            codegen::write(&program, arch, &mut stdout)
        }

        path => {
            let mut file = File::create(path)
                .with_context(|| format!("Failed to open for writing: {}", path))?;

            codegen::write(&program, arch, &mut file)
        }
    };

    result.context("Failed to emit generated code").into()
}

fn test_program() -> Program {
    let builtin_true = Rc::new(Function {
        name: String::from("builtin_true"),
        parameters: 0,
        body: FunctionBody::External,
    });

    let builtin_neg = Rc::new(Function {
        name: String::from("builtin_neg"),
        parameters: 1,
        body: FunctionBody::External,
    });

    let builtin_debug = Rc::new(Function {
        name: String::from("builtin_debug"),
        parameters: 1,
        body: FunctionBody::External,
    });

    let global_foo = Rc::new(Global(String::from("foo")));
    let global_bar = Rc::new(Global(String::from("bar")));

    let user_test = Rc::new(Function {
        name: String::from("user_test"),
        parameters: 9,
        body: FunctionBody::Generated {
            inner_locals: 1,
            instructions: vec![
                Instruction::LoadGlobal(Rc::clone(&global_foo), Local(9)),
                Instruction::Call {
                    target: Rc::clone(&builtin_debug),
                    arguments: vec![Local(9)],
                    output: None,
                },
                Instruction::Call {
                    target: Rc::clone(&builtin_neg),
                    arguments: vec![Local(9)],
                    output: Some(Local(9)),
                },
                Instruction::StoreGlobal(Local(9), Rc::clone(&global_foo)),
            ],
        },
    });

    let user_main = Rc::new(Function {
        name: String::from("user_main"),
        parameters: 0,
        body: FunctionBody::Generated {
            inner_locals: 9,
            instructions: vec![
                Instruction::Call {
                    target: Rc::clone(&builtin_true),
                    arguments: vec![],
                    output: Some(Local(0)),
                },
                Instruction::StoreGlobal(Local(0), Rc::clone(&global_foo)),
                Instruction::Label(Label(42)),
                Instruction::Call {
                    target: Rc::clone(&user_test),
                    arguments: vec![
                        Local(0),
                        Local(1),
                        Local(2),
                        Local(3),
                        Local(4),
                        Local(5),
                        Local(6),
                        Local(7),
                        Local(8),
                    ],
                    output: Some(Local(0)),
                },
                Instruction::Jump(Label(42)),
            ],
        },
    });

    Program {
        globals: vec![global_foo, global_bar],
        code: vec![user_test, user_main],
    }
}
