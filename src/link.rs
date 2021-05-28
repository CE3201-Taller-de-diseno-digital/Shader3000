use std::{
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    str::FromStr,
};

use crate::arch::Arch;
use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LinkerError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("Linker exited with status code {0:?}")]
    Failed(ExitStatus),
}

#[derive(Copy, Clone)]
pub enum Platform {
    Native,
    Esp8266,
}

#[cfg(not(target_arch = "x86_64"))]
error!("Native target is not x86-64");

impl Platform {
    pub fn arch(self) -> Arch {
        match self {
            Platform::Native => Arch::X86_64,
            Platform::Esp8266 => Arch::Xtensa,
        }
    }
}

impl FromStr for Platform {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "native" => Ok(Platform::Native),
            "esp8266" => Ok(Platform::Esp8266),
            _ => Err(()),
        }
    }
}

pub struct Linker(Child);

impl Linker {
    pub fn spawn<O: AsRef<Path>>(platform: Platform, output: &O) -> Result<Linker, LinkerError> {
        let params = platform.link_params();

        let mut library_path: PathBuf = "lib".into();
        library_path.push(params.name);

        Command::new(params.command)
            .args(params.extra_args)
            .arg("-L")
            .arg(&library_path)
            .arg("-o")
            .arg(output.as_ref())
            .args(&["-Wl,--gc-sections", "-xassembler", "-", "-lruntime"])
            .stdin(Stdio::piped())
            .spawn()
            .map(Linker)
            .map_err(LinkerError::Io)
    }

    pub fn stdin(&mut self) -> &mut ChildStdin {
        self.0.stdin.as_mut().unwrap()
    }

    pub fn finish(mut self) -> Result<(), LinkerError> {
        let status = self.0.wait().map_err(LinkerError::Io)?;
        if status.success() {
            Ok(())
        } else {
            Err(LinkerError::Failed(status))
        }
    }
}

struct Parameters {
    name: &'static str,
    command: &'static str,
    extra_args: &'static [&'static str],
}

impl Platform {
    fn link_params(self) -> Parameters {
        match self {
            Platform::Native => Parameters {
                name: "native",
                command: "gcc",
                extra_args: &["-pthread", "-ldl"],
            },

            Platform::Esp8266 => Parameters {
                name: "esp8266",
                command: "xtensa-lx106-elf-gcc",
                extra_args: &["-nostartfiles", "-Wl,-Tlink.x"],
            },
        }
    }
}
