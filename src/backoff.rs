use std::time::Duration as StdDuration;

pub struct Backoff {
    delay_millis: u64,
    max_delay_millis: u64,
    attempts: i32,
}

impl Backoff {
    pub fn new(initial_delay_millis: u64, max_delay_millis: u64) -> Self {
        Self {
            delay_millis: initial_delay_millis,
            max_delay_millis,
            attempts: 1,
        }
    }

    /// Retrieves the upcoming delay.
    pub fn next(&mut self) -> StdDuration {
        let delay_millis = self.delay_millis;
        self.delay_millis = self.max_delay_millis.min(delay_millis * 2);
        self.attempts += 1;
        StdDuration::from_millis(delay_millis + fastrand::u64(0..delay_millis))
    }

    pub fn attempts(&self) -> i32 {
        self.attempts
    }
}
