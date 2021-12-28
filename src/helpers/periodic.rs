use std::time::Instant;

use crate::StdDuration;

pub struct Periodic {
    interval: StdDuration,
    last_triggered_at: Instant,
}

impl Periodic {
    #[must_use]
    pub fn new(interval: StdDuration) -> Self {
        Self {
            interval,
            last_triggered_at: Instant::now(),
        }
    }

    #[must_use]
    pub fn should_trigger(&mut self) -> bool {
        let now = Instant::now();
        if now - self.last_triggered_at > self.interval {
            self.last_triggered_at = now;
            true
        } else {
            false
        }
    }
}
