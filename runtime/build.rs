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
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "xtensa" {
        xbuild_main()
    } else {
        hosted_main()
    }
}

fn xbuild_main() -> ExitCode {
    const PATH: &str = "src/esp8266/atomic/atomic.c";

    /*println!("cargo:rerun-if-changed={}", PATH);
    cc::Build::new()
        .compiler("xtensa-lx106-elf-gcc")
        .file(PATH)
        .compile("atomic_shim");*/

    ExitCode::SUCCESS
}

fn hosted_main() -> ExitCode {
    let xtensa_root: PathBuf = if let Ok(xtensa_root) = env::var("RUST_XTENSA") {
        xtensa_root.into()
    } else {
        eprintln!("Set the RUST_XTENSA environment variable");
        return ExitCode::FAILURE;
    };

    let profile = env::var("PROFILE").unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap())
        .canonicalize()
        .unwrap();
    let object_file = out_path.join("entry.o");

    let status = Command::new(env::var("RUSTC").unwrap())
        .args(&[
            "-O",
            "-C",
            "panic=abort",
            "--edition=2018",
            "--emit=obj",
            "-o",
            object_file.to_str().unwrap(),
            "src/entry.rs",
        ])
        .spawn()
        .expect("Failed to run rustc")
        .wait()
        .unwrap();

    if !status.success() {
        return ExitCode::FAILURE;
    }

    let archive_file = out_path.join("librt_entry.a");
    let status = Command::new("ar")
        .args(&[
            "rcs",
            archive_file.to_str().unwrap(),
            object_file.to_str().unwrap(),
        ])
        .spawn()
        .expect("Failed to run ar")
        .wait()
        .unwrap();

    if !status.success() {
        return ExitCode::FAILURE;
    }

    println!("cargo:rustc-link-search={}", out_path.to_str().unwrap());
    println!("cargo:rustc-link-lib=static=rt_entry");

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
