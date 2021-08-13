use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct TotalCrawlerMetrics {
    pub n_api_requests: Arc<AtomicU32>,
    pub hot: CrawlerMetrics,
    pub cold: CrawlerMetrics,

    start: Instant,
}

#[derive(Clone)]
pub struct CrawlerMetrics {
    pub n_accounts: Arc<AtomicUsize>,
    pub n_tanks: Arc<AtomicUsize>,
    pub last_account_id: Arc<AtomicI32>,
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
        let cold_tps = self.cold.n_tanks.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        let hot_tps = self.hot.n_tanks.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;
        self.start = Instant::now();

        log::info!(
            concat!(
                "RPS: {rps:.1}",
                " | ",
                "APS: {total_aps:.0} = ♨{hot_aps:.0} ❄{cold_aps:.0}",
                " | ",
                "TPS: {total_tps:.1} = ♨{hot_tps:.1} ❄{cold_tps:.1}",
                " | ",
                "♨#{last_hot_account_id} ❄#{last_cold_account_id}",
            ),
            rps = rps,
            total_aps = hot_aps + cold_aps,
            hot_aps = hot_aps,
            cold_aps = cold_aps,
            total_tps = hot_tps + cold_tps,
            hot_tps = hot_tps,
            cold_tps = cold_tps,
            last_hot_account_id = self.hot.last_account_id.load(Ordering::Relaxed),
            last_cold_account_id = self.cold.last_account_id.load(Ordering::Relaxed),
        );
    }
}

impl CrawlerMetrics {
    pub fn new() -> Self {
        Self {
            n_accounts: Arc::new(AtomicUsize::new(0)),
            n_tanks: Arc::new(AtomicUsize::new(0)),
            last_account_id: Arc::new(AtomicI32::new(0)),
        }
    }
}
