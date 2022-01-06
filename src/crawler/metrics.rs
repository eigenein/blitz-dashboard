use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use chrono::Utc;
use humantime::format_duration;
use tokio::sync::RwLock;

use crate::helpers::periodic::Periodic;
use crate::DateTime;

pub struct CrawlerMetrics {
    /// Scanned account count.
    n_accounts: u32,

    /// Inserted tank snapshot count.
    n_tanks: usize,

    n_battles: i32,

    /// Request count from the last `log()` call.
    last_request_count: u32,

    reset_instant: Instant,

    /// API request counter.
    request_counter: Arc<AtomicU32>,

    lags: Vec<u64>,

    log_trigger: Periodic,

    lag_percentile: usize,
}

impl CrawlerMetrics {
    pub fn new(
        request_counter: Arc<AtomicU32>,
        log_interval: StdDuration,
        lag_percentile: usize,
    ) -> Self {
        Self {
            last_request_count: request_counter.load(Ordering::Relaxed),
            request_counter,
            n_accounts: 0,
            n_tanks: 0,
            n_battles: 0,
            reset_instant: Instant::now(),
            lags: Vec::new(),
            log_trigger: Periodic::new(log_interval),
            lag_percentile,
        }
    }

    pub fn add_account(&mut self) {
        self.n_accounts += 1;
    }

    pub fn add_tanks(&mut self, last_battle_time: DateTime, n_tanks: usize) -> crate::Result {
        self.lags
            .push((Utc::now() - last_battle_time).num_seconds().try_into()?);
        self.n_tanks += n_tanks;
        Ok(())
    }

    pub fn add_battles(&mut self, n_battles: i32) {
        self.n_battles += n_battles;
    }

    pub async fn check(&mut self, auto_min_offset: &Option<Arc<RwLock<StdDuration>>>) {
        if self.log_trigger.should_trigger() {
            let lag = self.aggregate_and_log();
            if let Some(min_offset) = auto_min_offset {
                *min_offset.write().await = lag;
            }
            self.reset();
        }
    }

    fn reset(&mut self) {
        self.n_accounts = 0;
        self.n_tanks = 0;
        self.n_battles = 0;
        self.lags.clear();
        self.reset_instant = Instant::now();
        self.last_request_count = self.request_counter.load(Ordering::Relaxed);
    }

    fn lag(&mut self) -> StdDuration {
        if self.lags.is_empty() {
            return StdDuration::new(0, 0);
        }

        let index = self.lag_percentile * self.lags.len() / 100;
        let (_, secs, _) = self.lags.select_nth_unstable(index);
        StdDuration::from_secs(*secs)
    }

    fn aggregate_and_log(&mut self) -> StdDuration {
        let elapsed_secs = self.reset_instant.elapsed().as_secs_f64();
        let elapsed_mins = elapsed_secs / 60.0;
        let n_requests = self.request_counter.load(Ordering::Relaxed) - self.last_request_count;

        let lag = self.lag();
        let mut formatted_lag = format_duration(lag).to_string();
        formatted_lag.truncate(11);

        log::info!(
            "RPS: {:>4.1} | battles: {:>4} | L{}: {:>11} | NA: {:>4} | APM: {:5.1} | TPM: {:.2}",
            n_requests as f64 / elapsed_secs,
            self.n_battles,
            self.lag_percentile,
            formatted_lag,
            self.n_accounts,
            self.n_accounts as f64 / elapsed_mins,
            self.n_tanks as f64 / elapsed_mins,
        );

        lag
    }
}
