use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use humantime::format_duration;

pub struct CrawlerMetrics {
    /// Scanned account count.
    pub n_accounts: u32,

    /// Inserted tank snapshot count.
    pub n_tanks: usize,

    /// Last scanned account ID.
    pub last_account_id: i32,

    pub n_battles: i32,

    /// Request count from the last `log()` call.
    last_request_count: u32,

    reset_instant: Instant,

    /// API request counter.
    request_counter: Arc<AtomicU32>,

    lags: Vec<u64>,
}

impl CrawlerMetrics {
    pub fn new(request_counter: Arc<AtomicU32>) -> Self {
        Self {
            last_request_count: request_counter.load(Ordering::Relaxed),
            request_counter,
            n_accounts: 0,
            n_tanks: 0,
            last_account_id: 0,
            n_battles: 0,
            reset_instant: Instant::now(),
            lags: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.n_accounts = 0;
        self.n_tanks = 0;
        self.n_battles = 0;
        self.lags.clear();
        self.reset_instant = Instant::now();
        self.last_request_count = self.request_counter.load(Ordering::Relaxed);
    }

    pub fn push_lag(&mut self, secs: u64) {
        self.lags.push(secs);
    }

    pub fn lags(&mut self) -> (StdDuration, StdDuration) {
        if self.lags.is_empty() {
            return (StdDuration::default(), StdDuration::default());
        }

        let index = self.lags.len() / 2;
        let (_, secs, _) = self.lags.select_nth_unstable(index);
        let lag_p50 = StdDuration::from_secs(*secs);

        let index = self.lags.len() * 9 / 10;
        let (_, secs, _) = self.lags.select_nth_unstable(index);
        let lag_p90 = StdDuration::from_secs(*secs);

        (lag_p50, lag_p90)
    }

    pub fn log(&mut self) -> StdDuration {
        let elapsed_secs = self.reset_instant.elapsed().as_secs_f64();
        let n_requests = self.request_counter.load(Ordering::Relaxed) - self.last_request_count;

        let (lag_p50, lag_p90) = self.lags();
        log::info!(
            "RPS: {:>4.1} | battles: {:>4.0} | L50: {:>11} | L90: {:>11} | APS: {:5.1} | TPS: {:.2} | A: {:>9}",
            n_requests as f64 / elapsed_secs,
            self.n_battles,
            format_duration(lag_p50).to_string(),
            format_duration(lag_p90).to_string(),
            self.n_accounts as f64 / elapsed_secs,
            self.n_tanks as f64 / elapsed_secs,
            self.last_account_id,
        );

        self.reset();
        lag_p50
    }
}
