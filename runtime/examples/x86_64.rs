#![feature(global_asm)]
#![no_main]

#[cfg(not(target_arch = "x86_64"))]
error!("This example works only on x86_64");

use runtime as _;

global_asm!(include_str!("x86_64.s"));
