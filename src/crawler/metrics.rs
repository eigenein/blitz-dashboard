use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

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
    hot: Arc<Mutex<SubCrawlerMetrics>>,
    cold: Arc<Mutex<SubCrawlerMetrics>>,
    frozen: Arc<Mutex<SubCrawlerMetrics>>,
) -> crate::Result {
    loop {
        let start_instant = Instant::now();
        tokio::time::sleep(StdDuration::from_secs(20)).await;
        let elapsed_secs = start_instant.elapsed().as_secs_f64();

        let rps = request_counter.swap(0, Ordering::Relaxed) as f64 / elapsed_secs;

        let mut hot = hot.lock().await;
        let mut cold = cold.lock().await;
        let mut frozen = frozen.lock().await;

        let frozen_aps = frozen.n_accounts as f64 / elapsed_secs;
        let cold_aps = cold.n_accounts as f64 / elapsed_secs;
        let hot_aps = hot.n_accounts as f64 / elapsed_secs;

        let cold_tps = cold.n_tanks as f64 / elapsed_secs;
        let hot_tps = hot.n_tanks as f64 / elapsed_secs;

        let cold_lag_secs = cold.max_lag_secs;
        let hot_lag_secs = hot.max_lag_secs;

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
            last_hot_account_id = hot.last_account_id,
            last_cold_account_id = cold.last_account_id,
            last_frozen_account_id = frozen.last_account_id,
            hot_lag = humantime::format_duration(StdDuration::from_secs(hot_lag_secs)),
            cold_lag = humantime::format_duration(StdDuration::from_secs(cold_lag_secs)),
        );

        hot.reset();
        cold.reset();
        frozen.reset();
    }
}
