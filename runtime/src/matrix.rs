use crate::chrono::Ticks;
use core::ops::{Index, IndexMut, Not};

#[derive(Default)]
pub struct Display([[Light; 8]; 8]);

impl Display {
    #[allow(dead_code)]
    pub fn rows(&self) -> &[[Light; 8]; 8] {
        &self.0
    }

    #[allow(dead_code)]
    pub fn row_bits(&self, row: usize) -> u8 {
        self.0[row]
            .iter()
            .fold(0, |acc, light| acc << 1 | (light.state == State::On) as u8)
    }

    pub fn tick(&mut self) {
        for row in self.0.iter_mut() {
            for light in row.iter_mut() {
                if light.clock.cycle_each(light.interval) {
                    light.state = !light.state;
                }
            }
        }
    }
}

impl Index<(isize, isize)> for Display {
    type Output = Light;

    fn index(&self, (row, col): (isize, isize)) -> &Self::Output {
        check_indices(row, col);
        &self.0[row as usize][col as usize]
    }
}

impl IndexMut<(isize, isize)> for Display {
    fn index_mut(&mut self, (row, col): (isize, isize)) -> &mut Self::Output {
        check_indices(row, col);
        &mut self.0[row as usize][col as usize]
    }
}

#[derive(Default)]
pub struct Light {
    state: State,
    clock: Ticks,
    interval: Ticks,
}

impl Light {
    #[allow(dead_code)]
    pub fn state(&self) -> State {
        self.state
    }

    pub fn set(&mut self, state: State) {
        self.state = state;
    }

    pub fn blink(&mut self, interval: Ticks) {
        self.clock = interval;
        self.interval = interval;
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    On,
    Off,
}

impl State {
    pub fn from_bool(value: bool) -> Self {
        if value {
            State::On
        } else {
            State::Off
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State::Off
    }
}

impl Not for State {
    type Output = State;

    fn not(self) -> Self::Output {
        match self {
            State::On => State::Off,
            State::Off => State::On,
        }
    }
}

fn check_indices(row: isize, col: isize) {
    let valid = 0..8;
    assert!(
        valid.contains(&row) && valid.contains(&col),
        "Display matrix index [{}, {}] is out of bounds",
        row,
        col
    );
}
