use std::time::Duration as StdDuration;

pub struct Backoff {
    delay_millis: u64,
}

impl Backoff {
    pub fn new(initial_delay_millis: u64) -> Self {
        Self {
            delay_millis: initial_delay_millis,
        }
    }

    pub fn next(&mut self) -> StdDuration {
        let delay_millis = self.delay_millis;
        self.delay_millis *= 2;
        StdDuration::from_millis(delay_millis + fastrand::u64(0..delay_millis))
    }
}
