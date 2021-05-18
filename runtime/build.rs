#![feature(process_exitcode_placeholder)]

use nix::libc;
use std::{
    env,
    fs::File,
    os::unix::{io::AsRawFd, process::CommandExt},
    path::PathBuf,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let xtensa_root: PathBuf = if let Ok(xtensa_root) = env::var("RUST_XTENSA") {
        xtensa_root.into()
    } else {
        eprintln!("Set the RUST_XTENSA environment variable");
        return ExitCode::FAILURE;
    };

    let mut bin_path = xtensa_root.join("build");
    bin_path.push(env::var("HOST").unwrap());
    bin_path.push("stage2/bin");

    env::set_var("XARGO_RUST_SRC", xtensa_root.join("library"));
    env::set_var("RUSTC", bin_path.join("rustc"));
    env::set_var("RUSTDOC", bin_path.join("rustdoc"));

    restore_tty_output();

    let error = Command::new(env::var("CARGO").unwrap())
        .args(&[
            "xbuild",
            "-Z",
            "unstable-options",
            "--target",
            "xtensa-esp8266-none-elf",
            "--target-dir",
            "../xtarget",
            "--profile",
            &profile,
            "--package",
            "runtime",
        ])
        .exec();

    eprintln!("Failed to run cargo: {:?}", error);
    ExitCode::FAILURE
}

fn restore_tty_output() {
    let tty_file = File::create("/dev/tty").unwrap();
    nix::unistd::dup2(tty_file.as_raw_fd(), libc::STDERR_FILENO).unwrap();
    nix::unistd::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO).unwrap();
}
