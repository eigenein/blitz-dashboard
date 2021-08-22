use std::borrow::Cow;
use std::time::{Duration, Instant};

use log::Level;

/// Stopwatch to log a code block execution time.
pub struct Stopwatch {
    /// Logged message.
    message: Cow<'static, str>,

    start: Instant,

    /// Default logging level.
    log_level: Level,

    /// Optional execution time threshold.
    /// If exceeded, raises the logging level to warning.
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
        let elapsed = self.start.elapsed();
        log::log!(
            match self.threshold {
                Some(threshold) if elapsed >= threshold => Level::Warn,
                _ => self.log_level,
            },
            "{} in {}.",
            self.message,
            humantime::format_duration(elapsed),
        );
    }
}
