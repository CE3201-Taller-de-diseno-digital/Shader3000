#![feature(global_asm)]

#[cfg(not(target_arch = "x86_64"))]
error!("This example works only on x86_64");

use runtime::handover;

global_asm!(include_str!("x86_64.s"));

fn main() {
    handover();
}
