use std::sync::atomic::{AtomicU32, Ordering};

use crate::helpers::average::Average;
use crate::prelude::*;
use crate::wargaming;

pub struct CrawlerMetrics {
    log_interval: time::Duration,

    reset_instant: Instant,
    average_batch_fill_level: Average,
    start_request_count: u32,
    n_accounts: u32,
    last_account_id: wargaming::AccountId,
    total_lag: Duration,
}

impl CrawlerMetrics {
    pub fn new(request_counter: &AtomicU32, log_interval: time::Duration) -> Self {
        info!(?log_interval);
        Self {
            start_request_count: request_counter.load(Ordering::Relaxed),
            n_accounts: 0,
            last_account_id: 0,
            reset_instant: Instant::now(),
            average_batch_fill_level: Average::default(),
            log_interval,
            total_lag: Duration::zero(),
        }
    }

    pub fn add_account(&mut self, account: &database::AccountSnapshot) {
        self.n_accounts += 1;
        self.last_account_id = account.account_id;
        if account.last_battle_time.timestamp() != 0 {
            self.total_lag = self.total_lag + (now() - account.last_battle_time);
        }
    }

    pub fn add_batch(&mut self, batch_len: usize, matched_len: usize) {
        self.average_batch_fill_level
            .push(matched_len as f64 / batch_len as f64);
    }

    pub fn check(&mut self, request_counter: &AtomicU32) -> bool {
        let now = Instant::now();
        let elapsed = self.reset_instant.elapsed();
        if elapsed >= self.log_interval {
            let request_counter = request_counter.load(Ordering::Relaxed);
            self.log(request_counter, elapsed);
            self.reset(request_counter, now);
            true
        } else {
            false
        }
    }

    fn log(&self, request_counter: u32, elapsed: time::Duration) {
        let elapsed_secs = elapsed.as_secs_f64();
        let elapsed_mins = elapsed_secs / 60.0;
        let n_requests = request_counter - self.start_request_count;

        info!(
            rps = %format!("{:.1}", n_requests as f64 / elapsed_secs),
            fill = %format!("{:.1}%", self.average_batch_fill_level.average() * 100.0),
            apm = %format!("{:.0}", self.n_accounts as f64 / elapsed_mins),
            lag_hrs = %format!("{:.1}", self.lag_hours()),
            id = self.last_account_id,
        );
    }

    fn reset(&mut self, request_counter: u32, now: Instant) {
        self.reset_instant = now;
        self.average_batch_fill_level = Default::default();
        self.start_request_count = request_counter;
        self.n_accounts = 0;
        self.total_lag = Duration::zero();
    }

    fn lag_hours(&self) -> f64 {
        if self.n_accounts != 0 {
            (self.total_lag.num_seconds() as f64) / (self.n_accounts as f64) / 3600.0
        } else {
            0f64
        }
    }
}
