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

    pub n_battles: i32,

    lags: Vec<u64>,

    cf_errors: Vec<f64>,
}

impl SubCrawlerMetrics {
    pub fn reset(&mut self) {
        self.n_accounts = 0;
        self.n_tanks = 0;
        self.n_battles = 0;
        self.cf_errors.clear();
        self.lags.clear();
    }

    pub fn push_lag(&mut self, secs: u64) {
        self.lags.push(secs);
    }

    pub fn push_cf_error(&mut self, error: f64) {
        self.cf_errors.push(error);
    }

    pub fn cf_error(&self) -> (f64, f64) {
        let n = self.cf_errors.len().max(1);
        let mean = self.cf_errors.iter().sum::<f64>() / n as f64;
        let variance = self
            .cf_errors
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / ((n - 1).max(1)) as f64;
        (mean, variance.sqrt())
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
}

pub async fn log_metrics(
    request_counter: Arc<AtomicU32>,
    metrics: Vec<Arc<Mutex<SubCrawlerMetrics>>>,
) -> crate::Result {
    loop {
        let start_instant = Instant::now();
        let n_requests_start = request_counter.load(Ordering::Relaxed);
        tokio::time::sleep(StdDuration::from_secs(60)).await;
        let elapsed_secs = start_instant.elapsed().as_secs_f64();
        let n_requests = request_counter.load(Ordering::Relaxed) - n_requests_start;

        log::info!("Total RPS: {:.1}", n_requests as f64 / elapsed_secs);

        for (i, metrics) in metrics.iter().enumerate() {
            let mut metrics = metrics.lock().await;
            let (lag_p50, lag_p90) = metrics.lags();
            let (cfe_mean, cfe_std) = metrics.cf_error();
            log::info!(
                "Sub-crawler #{} | L50: {:>11} | L90: {:>11} | APS: {:5.1} | TPS: {:.2} | A: {:>9} | CFME: {:>+.3} ±{:>.3}",
                i,
                format_duration(lag_p50).to_string(),
                format_duration(lag_p90).to_string(),
                metrics.n_accounts as f64 / elapsed_secs,
                metrics.n_tanks as f64 / elapsed_secs,
                metrics.last_account_id,
                cfe_mean,
                cfe_std,
            );
            metrics.reset();
        }
    }
}
