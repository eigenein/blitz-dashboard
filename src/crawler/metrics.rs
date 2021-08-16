use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use humantime::format_duration;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct SubCrawlerMetrics {
    /// Scanned account count.
    pub n_accounts: u32,

    /// Inserted tank snapshot count.
    pub n_tanks: usize,

    /// Last scanned account ID.
    pub last_account_id: i32,

    /// Maximum lag – time after an account last battle time and now – in seconds.
    /// Note: by the time a sub-crawler scans an account, it may have already played more than 1 battle.
    /// So, the real maximum lag may be greater than that.
    /// It's also useful to check the full-scan time for a specific sub-crawler («hot» or «cold»).
    pub max_lag_secs: u64,
}

impl SubCrawlerMetrics {
    pub fn reset(&mut self) {
        self.n_accounts = 0;
        self.n_tanks = 0;
        self.last_account_id = 0;
        self.max_lag_secs = 0;
    }
}

pub async fn log_metrics(
    request_counter: Arc<AtomicU32>,
    metrics: Vec<Arc<Mutex<SubCrawlerMetrics>>>,
) -> crate::Result {
    loop {
        let start_instant = Instant::now();
        let n_requests_start = request_counter.load(Ordering::Relaxed);
        tokio::time::sleep(StdDuration::from_secs(20)).await;
        let elapsed_secs = start_instant.elapsed().as_secs_f64();
        let n_requests = request_counter.load(Ordering::Relaxed) - n_requests_start;

        log::info!("Total RPS: {:.1}", n_requests as f64 / elapsed_secs);

        for (i, metrics) in metrics.iter().enumerate() {
            let mut metrics = metrics.lock().await;
            log::info!(
                "Sub-crawler #{} | max lag: {} | APS: {:.1} | TPS: {:.2} | at: #{}",
                i,
                format_duration(StdDuration::from_secs(metrics.max_lag_secs)),
                metrics.n_accounts as f64 / elapsed_secs,
                metrics.n_tanks as f64 / elapsed_secs,
                metrics.last_account_id,
            );
            metrics.reset();
        }
    }
}
