use clap::{self, crate_authors, crate_description, crate_version};
use compiler::{
    codegen::{self, Architecture},
    ir::*,
};

use std::rc::Rc;

fn main() {
    let _args = clap::App::new("AnimationLed compiler")
        .author(crate_authors!())
        .about(crate_description!())
        .version(crate_version!())
        .get_matches();

    let program = test_program();
    let mut stdout = std::io::stdout();

    codegen::write(&program, Architecture::Xtensa, &mut stdout).unwrap();
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
