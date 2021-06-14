//! Punto de entrada ("driver").
//!
//! Este módula orquesta las diferentes fases del proceso de
//! compilación y expone una CLI.

use anyhow::{self, bail, Context};
use clap::{self, crate_version, Arg};
use compiler::{
    ir::*,
    link::{LinkOptions, Linker, Platform},
    target,
};

use std::{fs::File, rc::Rc, str::FromStr};

fn main() -> anyhow::Result<()> {
    // Parsing de CLI
    let args = clap::App::new("AnimationLed compiler")
        .version(crate_version!())
        .arg(
            Arg::new("target")
                .short('t')
                .long("target")
                .value_name("PLATFORM")
                .takes_value(true)
                .default_value("native")
                .possible_values(&["native", "esp8266"])
                .about("Target platform"),
        )
        .arg(
            Arg::new("asm")
                .short('S')
                .about("Generate assembly instead of linking"),
        )
        .arg(Arg::new("strip").short('s').about("Strip executables"))
        .arg(
            Arg::new("output")
                .short('o')
                .takes_value(true)
                .required(true)
                .value_name("FILE")
                .about("Output file ('-' along with -S for stdout)"),
        )
        .get_matches();

    // Se extraen argumentos necesarios
    let platform = args.value_of("target").unwrap();
    let platform = Platform::from_str(&platform).expect("main.rs allowed a bad target");
    let arch = platform.arch();
    let asm = args.is_present("asm");
    let output = args.value_of("output").unwrap();

    let program = test_program();
    match (asm, output) {
        // Salida a stdout sin enlazado
        (true, "-") => {
            let mut stdout = std::io::stdout();
            target::emit(&program, arch, &mut stdout).context("Failed to emit to stdin")?;
        }

        // Salida a archivo sin enlazado
        (true, path) => {
            let mut file = File::create(path)
                .with_context(|| format!("Failed to open for writing: {}", path))?;

            target::emit(&program, arch, &mut file)
                .with_context(|| format!("Failed to emit to file: {}", path))?;
        }

        // Salida a stdout con enlazado
        (false, "-") => bail!("Refusing to write executable to stdout"),

        // Salida a archivo con enlazado
        (false, path) => {
            let mut options = LinkOptions::empty();
            if args.is_present("strip") {
                options |= LinkOptions::STRIP;
            }

            let mut linker = Linker::spawn(platform, &path, options).context("Failed to link")?;
            target::emit(&program, arch, linker.stdin())
                .context("Failed to emit assembly to assembler")?;

            linker
                .finish()
                .with_context(|| format!("Failed to generate executable: {}", path))?;
        }
    };

    Ok(())
}

fn test_program() -> Program {
    // Este es un programa de prueba para mientras no se haya terminado la
    // pipeline lexer->parser->magia->ir->asm->link. Debería eliminarse
    // eventualmente.

    let builtin_inc = Function::External("builtin_inc");
    let builtin_delay_seg = Function::External("builtin_delay_seg");
    let builtin_debug = Function::External("builtin_debug");
    let builtin_print_led = Function::External("builtin_printled");
    let builtin_blink_mil = Function::External("builtin_blink_mil");

    let user_main = GeneratedFunction {
        name: Rc::new(String::from("user_main")),
        parameters: 0,
        body: vec![
            Instruction::LoadConst(0, Local(0)),
            Instruction::LoadConst(500, Local(1)),
            Instruction::LoadConst(1, Local(2)),
            Instruction::Call {
                target: builtin_blink_mil,
                arguments: vec![Local(0), Local(0), Local(1), Local(2)],
                output: None,
            },
            Instruction::SetLabel(Label(0)),
            Instruction::Call {
                target: builtin_print_led,
                arguments: vec![Local(0), Local(0), Local(2)],
                output: None,
            },
            Instruction::Call {
                target: builtin_inc,
                arguments: vec![Local(0)],
                output: Some(Local(0)),
            },
            Instruction::Call {
                target: builtin_debug,
                arguments: vec![Local(0)],
                output: None,
            },
            Instruction::Call {
                target: builtin_delay_seg,
                arguments: vec![Local(0)],
                output: None,
            },
            Instruction::Jump(Label(0)),
        ],
    };

    Program {
        globals: vec![],
        code: vec![user_main],
    }
}
