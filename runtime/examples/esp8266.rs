#![feature(global_asm)]
#![no_main]
#![no_std]

#[cfg(not(target_arch = "xtensa"))]
error!("This example works only on xl106/esp8266");

use runtime as _;

global_asm!(include_str!("esp8266.s"));
