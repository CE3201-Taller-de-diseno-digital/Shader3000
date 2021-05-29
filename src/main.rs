use anyhow::{self, bail, Context};
use clap::{self, crate_version, Arg};
use compiler::{
    ir::*,
    link::{LinkOptions, Linker, Platform},
    target,
};

use std::{fs::File, rc::Rc, str::FromStr};

fn main() -> anyhow::Result<()> {
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

    let platform = args.value_of("target").unwrap();
    let platform = Platform::from_str(&platform).expect("main.rs allowed a bad target");
    let arch = platform.arch();

    let asm = args.is_present("asm");
    let output = args.value_of("output").unwrap();

    let program = test_program();
    match (asm, output) {
        (true, "-") => {
            let mut stdout = std::io::stdout();
            target::emit(&program, arch, &mut stdout).context("Failed to emit to stdin")?;
        }

        (true, path) => {
            let mut file = File::create(path)
                .with_context(|| format!("Failed to open for writing: {}", path))?;

            target::emit(&program, arch, &mut file)
                .with_context(|| format!("Failed to emit to file: {}", path))?;
        }

        (false, "-") => bail!("Refusing to write executable to stdout"),

        (false, path) => {
            let mut options = LinkOptions::empty();
            if args.is_present("strip") {
                options |= LinkOptions::STRIP;
            }

            let mut linker = Linker::spawn(platform, &path, options).context("Failed to link")?;
            target::emit(&program, arch, linker.stdin())
                .context("Failed to emit through linker")?;

            linker
                .finish()
                .with_context(|| format!("Failed to generate executable: {}", path))?;
        }
    };

    Ok(())
}

fn test_program() -> Program {
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
            Instruction::LoadConst(0, Local(0)),
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
