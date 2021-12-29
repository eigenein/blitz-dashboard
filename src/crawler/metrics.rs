use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use arc_swap::ArcSwap;
use chrono::Utc;
use humantime::format_duration;

use crate::helpers::periodic::Periodic;
use crate::DateTime;

pub struct CrawlerMetrics {
    /// Scanned account count.
    n_accounts: u32,

    /// Inserted tank snapshot count.
    n_tanks: usize,

    /// Last scanned account ID.
    last_account_id: i32,

    n_battles: i32,

    /// Request count from the last `log()` call.
    last_request_count: u32,

    reset_instant: Instant,

    /// API request counter.
    request_counter: Arc<AtomicU32>,

    lags: Vec<u64>,

    log_trigger: Periodic,
}

impl CrawlerMetrics {
    pub fn new(request_counter: Arc<AtomicU32>, log_interval: StdDuration) -> Self {
        Self {
            last_request_count: request_counter.load(Ordering::Relaxed),
            request_counter,
            n_accounts: 0,
            n_tanks: 0,
            last_account_id: 0,
            n_battles: 0,
            reset_instant: Instant::now(),
            lags: Vec::new(),
            log_trigger: Periodic::new(log_interval),
        }
    }

    pub fn add_account(&mut self, account_id: i32) {
        self.last_account_id = account_id;
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

    pub fn check(&mut self, auto_min_offset: &Option<Arc<ArcSwap<StdDuration>>>) {
        if self.log_trigger.should_trigger() {
            let lag_50 = self.aggregate_and_log();
            if let Some(min_offset) = auto_min_offset {
                min_offset.swap(Arc::new(lag_50));
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

    fn lags(&mut self) -> (StdDuration, StdDuration) {
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

    fn aggregate_and_log(&mut self) -> StdDuration {
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

        lag_p50
    }
}
