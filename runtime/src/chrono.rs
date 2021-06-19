pub use core::time::Duration;

#[derive(Copy, Clone, Default)]
pub struct Ticks(usize);

impl Ticks {
    pub const fn from_duration(duration: Duration) -> Self {
        Ticks(crate::sys::tick_count_for(duration))
    }

    #[allow(dead_code)]
    pub fn done(self) -> bool {
        self.0 == 0
    }

    #[allow(dead_code)]
    pub fn countdown(&mut self) {
        match self {
            Ticks(0) => (),
            Ticks(ticks) => *ticks -= 1,
        }
    }

    pub fn cycle_each(&mut self, interval: Ticks) -> bool {
        match (self, interval) {
            (_, Ticks(0)) => false,

            (Ticks(ticks), Ticks(interval)) if *ticks <= 1 => {
                *ticks = interval;
                true
            }

            (ticks, _) => {
                ticks.0 -= 1;
                false
            }
        }
    }
}
