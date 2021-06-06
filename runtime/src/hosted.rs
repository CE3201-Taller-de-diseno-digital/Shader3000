//! Implementación de `runtime::sys` cuando se dispone de
//! un sistema operativo huésped.
//!
//! Naturalmente, esto es mucho más sencillo y trivial que
//! implementar las mismas operaciones para plataformas
//! embebidas y `#![no_std]`.

use lazy_static::lazy_static;

use std::{
    fmt::Write,
    sync::{Mutex, MutexGuard},
};

use crate::{
    chrono::{Duration, Ticks},
    matrix::{Display, State},
};

/// Imprime un mensaje de depuración.
macro_rules! sys_debug {
    ($($b:tt)*) => {
        println!($($b)*)
    }
}

/// Detiene el programa durante una cantidad de tiempo.
pub fn delay(duration: Duration) {
    std::thread::sleep(duration);
}

pub const fn tick_count_for(duration: Duration) -> usize {
    duration.as_millis() as usize / 10
}

pub fn with_display<F, R>(callback: F) -> R
where
    F: FnOnce(&mut Display) -> R,
{
    let mut display = lock();
    callback(&mut display)
}

lazy_static! {
    static ref DISPLAY: Mutex<Display> = Mutex::new(Display::default());
}

fn lock() -> MutexGuard<'static, Display> {
    lazy_static! {
        static ref CLOCK_THREAD: () = {
            std::thread::spawn(clock_main);
        };
    }

    lazy_static::initialize(&DISPLAY);
    lazy_static::initialize(&CLOCK_THREAD);

    DISPLAY.lock().unwrap()
}

fn clock_main() {
    let mut draw_clock = Ticks::default();
    const DRAW_TICKS: Ticks = Ticks::from_duration(Duration::from_millis(50));

    loop {
        delay(Duration::from_millis(10));

        let mut display = DISPLAY.lock().unwrap();
        display.tick();

        if draw_clock.cycle_each(DRAW_TICKS) {
            redraw(&display);
        }
    }
}

fn redraw(display: &Display) {
    use ansi_escapes::{CursorUp, EraseLines};

    let mut output = String::new();

    output.push_str("\n\n\n\n\n\n\n");
    write!(&mut output, "{}", EraseLines(8)).unwrap();

    for row in display.rows().iter() {
        for light in row {
            let symbol = match light.state() {
                State::On => '●',
                State::Off => '○',
            };

            output.push(symbol);
        }

        output.push('\n');
    }

    print!("{}{}", output, CursorUp(8));
}
