use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

pub struct CrawlerMetrics {
    pub n_api_requests: Arc<AtomicU32>,

    pub hot: SubCrawlerMetrics,
    pub cold: SubCrawlerMetrics,
    pub frozen: SubCrawlerMetrics,

    start: Instant,
}

#[derive(Clone)]
pub struct SubCrawlerMetrics {
    /// Updated account count.
    pub n_accounts: Arc<AtomicU32>,

    /// Inserted tank snapshot count.
    pub n_tanks: Arc<AtomicU32>,

    /// Last scanned account ID.
    pub last_account_id: Arc<AtomicI32>,

    /// Maximum lag – time after an account last battle time and now – in seconds.
    /// Note: by the time a sub-crawler scans an account, it may have already played more than 1 battle.
    /// So, the real maximum lag may be greater than that.
    /// It's also useful to check the full-scan time for a specific sub-crawler («hot» or «cold»).
    pub max_lag_secs: Arc<AtomicU64>,
}

impl CrawlerMetrics {
    pub fn new() -> Self {
        Self {
            n_api_requests: Arc::new(AtomicU32::new(0)),
            start: Instant::now(),
            hot: SubCrawlerMetrics::new(),
            cold: SubCrawlerMetrics::new(),
            frozen: SubCrawlerMetrics::new(),
        }
    }

    /// Logs the current metrics and resets the counters.
    pub fn log(&mut self) {
        let elapsed_secs = self.start.elapsed().as_secs_f64();
        let rps = self.n_api_requests.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;

        let frozen_aps = self.frozen.n_accounts.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let cold_aps = self.cold.n_accounts.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let hot_aps = self.hot.n_accounts.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;

        let cold_tps = self.cold.n_tanks.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let hot_tps = self.hot.n_tanks.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;

        let cold_lag_secs = self.cold.max_lag_secs.swap(0, Ordering::Relaxed);
        let hot_lag_secs = self.hot.max_lag_secs.swap(0, Ordering::Relaxed);

        self.start = Instant::now();

        log::info!(
            concat!(
                "RPS: {rps:.1}",
                " | ",
                "APS: {hot_aps:.0} - {cold_aps:.0} - {frozen_aps:.0}",
                " | ",
                "TPS: {hot_tps:.1} - {cold_tps:.2}",
                " | ",
                "max lag: {hot_lag} - {cold_lag}",
                " | ",
                "#{last_hot_account_id} - #{last_cold_account_id} - #{last_frozen_account_id}",
            ),
            rps = rps,
            hot_aps = hot_aps,
            cold_aps = cold_aps,
            frozen_aps = frozen_aps,
            hot_tps = hot_tps,
            cold_tps = cold_tps,
            last_hot_account_id = self.hot.last_account_id.load(Ordering::Relaxed),
            last_cold_account_id = self.cold.last_account_id.load(Ordering::Relaxed),
            last_frozen_account_id = self.frozen.last_account_id.load(Ordering::Relaxed),
            hot_lag = humantime::format_duration(StdDuration::from_secs(hot_lag_secs)),
            cold_lag = humantime::format_duration(StdDuration::from_secs(cold_lag_secs)),
        );
    }
}

impl SubCrawlerMetrics {
    pub fn new() -> Self {
        Self {
            n_accounts: Arc::new(AtomicU32::new(0)),
            n_tanks: Arc::new(AtomicU32::new(0)),
            last_account_id: Arc::new(AtomicI32::new(0)),
            max_lag_secs: Arc::new(AtomicU64::new(0)),
        }
    }
}
