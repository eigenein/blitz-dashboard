use std::borrow::Cow;
use std::time::{Duration, Instant};

use log::Level;

pub struct Stopwatch {
    message: Cow<'static, str>,
    start: Instant,
    log_level: Level,
    threshold: Option<Duration>,
}

impl Stopwatch {
    pub fn new<M: Into<Cow<'static, str>>>(message: M) -> Self {
        Self {
            message: message.into(),
            log_level: Level::Trace,
            start: Instant::now(),
            threshold: None,
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.log_level = level;
        self
    }

    pub fn threshold_millis(mut self, millis: u64) -> Self {
        self.threshold = Some(Duration::from_millis(millis));
        self
    }
}

impl Drop for Stopwatch {
    fn drop(&mut self) {
        let elapsed = Instant::now() - self.start;
        log::log!(
            match self.threshold {
                Some(threshold) if elapsed >= threshold => Level::Warn,
                _ => self.log_level,
            },
            "{} in {:?}.",
            self.message,
            elapsed,
        );
    }
}
