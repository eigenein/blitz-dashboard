use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::helpers::average::Average;
use crate::prelude::*;

pub struct CrawlerMetrics {
    pub average_batch_size: Average,
    pub average_batch_fill_level: Average,
    pub start_instant: Instant,
    pub lag_percentile: usize,

    start_request_count: u32,
    n_accounts: u32,
    last_account_id: i32,
    lags: Vec<u64>,
}

impl CrawlerMetrics {
    pub fn new(request_counter: &Arc<AtomicU32>, lag_percentile: usize) -> Self {
        Self {
            start_request_count: request_counter.load(Ordering::Relaxed),
            n_accounts: 0,
            last_account_id: 0,
            start_instant: Instant::now(),
            lags: Vec::new(),
            lag_percentile,
            average_batch_size: Average::default(),
            average_batch_fill_level: Average::default(),
        }
    }

    pub fn add_account(&mut self, account_id: i32) {
        self.n_accounts += 1;
        self.last_account_id = account_id;
    }

    pub fn add_lag_from(&mut self, last_battle_time: DateTime) -> Result {
        self.lags
            .push((Utc::now() - last_battle_time).num_seconds().try_into()?);
        Ok(())
    }

    pub fn add_batch(&mut self, batch_len: usize, matched_len: usize) {
        let batch_len = batch_len as f64;
        self.average_batch_size.push(batch_len);
        self.average_batch_fill_level
            .push(matched_len as f64 / batch_len);
    }

    pub async fn finalise(&mut self, request_counter: &Arc<AtomicU32>) -> Self {
        let elapsed_secs = self.start_instant.elapsed().as_secs_f64();
        let elapsed_mins = elapsed_secs / 60.0;
        let n_requests = request_counter.load(Ordering::Relaxed) - self.start_request_count;
        let lag = self.lag();

        info!(
            rps = %format!("{:.1}", n_requests as f64 / elapsed_secs),
            batch_size = %format!("{:.1}", self.average_batch_size.average()),
            batch_fill = %format!("{:.2}%", self.average_batch_fill_level.average() * 100.0),
            apm = %format!("{:.0}", self.n_accounts as f64 / elapsed_mins),
            ?lag,
            account_id = self.last_account_id,
        );

        Self::new(request_counter, self.lag_percentile)
    }

    fn lag(&mut self) -> StdDuration {
        if self.lags.is_empty() {
            return StdDuration::new(0, 0);
        }

        let index = self.lag_percentile * self.lags.len() / 100;
        let (_, secs, _) = self.lags.select_nth_unstable(index);
        StdDuration::from_secs(*secs)
    }
}
