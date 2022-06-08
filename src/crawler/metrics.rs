use std::sync::atomic::{AtomicU32, Ordering};

use crate::helpers::average::Average;
use crate::prelude::*;
use crate::tracing::format_duration;

pub struct CrawlerMetrics {
    lag_percentile: usize,
    log_interval: StdDuration,

    reset_instant: Instant,
    average_batch_fill_level: Average,
    start_request_count: u32,
    n_accounts: u32,
    last_account_id: i32,
    lags: Vec<u64>,
}

impl CrawlerMetrics {
    pub fn new(
        request_counter: &AtomicU32,
        lag_percentile: usize,
        log_interval: StdDuration,
    ) -> Self {
        Self {
            start_request_count: request_counter.load(Ordering::Relaxed),
            n_accounts: 0,
            last_account_id: 0,
            reset_instant: Instant::now(),
            lags: Vec::new(),
            lag_percentile,
            average_batch_fill_level: Average::default(),
            log_interval,
        }
    }

    pub fn add_account(&mut self, account_id: i32) {
        self.n_accounts += 1;
        self.last_account_id = account_id;
    }

    pub fn add_lag_from(&mut self, last_battle_time: DateTime) -> Result {
        self.lags
            .push((Utc::now() - last_battle_time).num_seconds().try_into()?);
        Ok(())
    }

    pub fn add_batch(&mut self, batch_len: usize, matched_len: usize) {
        self.average_batch_fill_level
            .push(matched_len as f64 / batch_len as f64);
    }

    pub fn check(&mut self, request_counter: &AtomicU32) {
        let now = Instant::now();
        let elapsed = now - self.reset_instant;
        if elapsed >= self.log_interval {
            let request_counter = request_counter.load(Ordering::Relaxed);
            self.log(request_counter, elapsed);
            self.reset(request_counter, now);
        }
    }

    fn log(&mut self, request_counter: u32, elapsed: StdDuration) {
        let elapsed_secs = elapsed.as_secs_f64();
        let elapsed_mins = elapsed_secs / 60.0;
        let n_requests = request_counter - self.start_request_count;
        let lag = self.lag();

        info!(
            rps = %format!("{:.1}", n_requests as f64 / elapsed_secs),
            fill = %format!("{:.1}%", self.average_batch_fill_level.average() * 100.0),
            apm = %format!("{:.0}", self.n_accounts as f64 / elapsed_mins),
            lag = format_duration(lag).as_str(),
            aid = self.last_account_id,
        );
    }

    fn reset(&mut self, request_counter: u32, now: Instant) {
        self.reset_instant = now;
        self.average_batch_fill_level = Default::default();
        self.start_request_count = request_counter;
        self.n_accounts = 0;
        self.lags.clear();
    }

    fn lag(&mut self) -> StdDuration {
        if self.lags.is_empty() {
            return StdDuration::new(0, 0);
        }

        let index = self.lag_percentile * self.lags.len() / 100;
        let (_, secs, _) = self.lags.select_nth_unstable(index);
        StdDuration::from_secs(*secs)
    }
}
