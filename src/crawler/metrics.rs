use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

pub struct TotalCrawlerMetrics {
    pub n_api_requests: Arc<AtomicU32>,
    pub hot: CrawlerMetrics,
    pub cold: CrawlerMetrics,

    start: Instant,
}

#[derive(Clone)]
pub struct CrawlerMetrics {
    pub n_accounts: Arc<AtomicU32>,
    pub n_updated_accounts: Arc<AtomicU64>,
    pub n_tanks: Arc<AtomicU32>,
    pub last_account_id: Arc<AtomicI32>,
    pub total_lag_secs: Arc<AtomicU64>,
}

impl TotalCrawlerMetrics {
    pub fn new() -> Self {
        Self {
            n_api_requests: Arc::new(AtomicU32::new(0)),
            start: Instant::now(),
            hot: CrawlerMetrics::new(),
            cold: CrawlerMetrics::new(),
        }
    }

    pub fn log(&mut self) {
        let elapsed_secs = self.start.elapsed().as_secs_f64();
        let rps = self.n_api_requests.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let cold_aps = self.cold.n_accounts.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let hot_aps = self.hot.n_accounts.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let total_aps = hot_aps + cold_aps;
        let cold_tps = self.cold.n_tanks.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let hot_tps = self.hot.n_tanks.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let cold_lag = self.cold.average_lag();
        let hot_lag = self.hot.average_lag();

        self.start = Instant::now();

        log::info!(
            concat!(
                "RPS: {rps:.1} V: {velocity:.0}%",
                " | ",
                "APS: 🔥{hot_aps:.0} 🧊{cold_aps:.0}",
                " | ",
                "TPS: 🔥{hot_tps:.1} 🧊{cold_tps:.2}",
                " | ",
                "lag: 🔥{hot_lag} 🧊{cold_lag}",
                " | ",
                "🔥#{last_hot_account_id} 🧊#{last_cold_account_id}",
            ),
            rps = rps,
            velocity = total_aps / rps,
            hot_aps = hot_aps,
            cold_aps = cold_aps,
            hot_tps = hot_tps,
            cold_tps = cold_tps,
            last_hot_account_id = self.hot.last_account_id.load(Ordering::Relaxed),
            last_cold_account_id = self.cold.last_account_id.load(Ordering::Relaxed),
            hot_lag = humantime::format_duration(hot_lag),
            cold_lag = humantime::format_duration(cold_lag),
        );
    }
}

impl CrawlerMetrics {
    pub fn new() -> Self {
        Self {
            n_accounts: Arc::new(AtomicU32::new(0)),
            n_updated_accounts: Arc::new(AtomicU64::new(0)),
            n_tanks: Arc::new(AtomicU32::new(0)),
            last_account_id: Arc::new(AtomicI32::new(0)),
            total_lag_secs: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn average_lag(&self) -> StdDuration {
        StdDuration::from_secs(
            self.total_lag_secs.swap(0, Ordering::Relaxed)
                / self.n_updated_accounts.swap(0, Ordering::Relaxed).max(1),
        )
    }
}
