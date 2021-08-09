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
        let elapsed = self.start.elapsed();
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

pub struct RpsCounter {
    tag: &'static str,
    counter: i32,
    counter_start: Instant,
    counter_threshold: i32,
}

impl RpsCounter {
    pub fn new(tag: &'static str, counter_threshold: i32) -> Self {
        Self {
            tag,
            counter: 0,
            counter_start: Instant::now(),
            counter_threshold,
        }
    }

    pub fn reset(&mut self) {
        self.counter = 0;
        self.counter_start = Instant::now();
    }

    pub fn increment(&mut self) {
        self.counter += 1;
        let elapsed = self.counter_start.elapsed().as_secs_f64();
        if self.counter >= self.counter_threshold || elapsed >= 15.0 {
            log::info!("{}: {:.1} RPS.", self.tag, self.counter as f64 / elapsed);
            self.reset();
        }
    }
}
