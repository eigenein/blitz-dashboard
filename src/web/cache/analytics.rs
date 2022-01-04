use std::sync::Arc;
use std::time::Instant;

use ahash::AHashMap;
use chrono::{Duration, TimeZone, Utc};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tokio::spawn;
use tokio::sync::RwLock;
use tracing::{error, instrument, warn};

use crate::trainer::dataset::{TwoTuple, XReadResponse, STREAM_KEY as DATASET_STREAM_KEY};
use crate::trainer::stream_entry::StreamEntry;
use crate::wargaming::tank_id::TankId;
use crate::{format_elapsed, DateTime};

#[derive(Clone)]
pub struct Analytics {
    pub win_rates: Arc<RwLock<AHashMap<TankId, f64>>>,

    triggered_at: Arc<RwLock<DateTime>>,
    redis: MultiplexedConnection,
    time_to_live: Duration,
    time_span: Duration,
}

impl Analytics {
    pub fn new(redis: MultiplexedConnection, time_span: Duration, time_to_live: Duration) -> Self {
        Self {
            redis,
            time_span,
            time_to_live,
            win_rates: Arc::new(RwLock::new(AHashMap::new())),
            triggered_at: Arc::new(RwLock::new(Utc.timestamp(0, 0))),
        }
    }

    pub async fn trigger_refresh(&self) {
        let expiry_time = *self.triggered_at.read().await + self.time_to_live;

        if Utc::now() > expiry_time {
            *self.triggered_at.write().await = Utc::now();
            let mut this = self.clone();
            spawn(async move {
                if let Err(error) = this.refresh().await {
                    error!("failed to refresh the analytics: {:#}", error);
                }
            });
        }
    }

    #[instrument(level = "debug", skip_all)]
    async fn refresh(&mut self) -> crate::Result {
        tracing::info!("refreshing the analytics…");
        let start_instant = Instant::now();

        let since = Utc::now() - self.time_span;
        let start_id = since.timestamp_millis().to_string();

        let mut response: XReadResponse =
            self.redis.xread(&[DATASET_STREAM_KEY], &[start_id]).await?;
        let entries = match response.pop() {
            Some(TwoTuple(_, entries)) => entries,
            None => {
                warn!("no streams in the response");
                return Ok(());
            }
        };

        let since_timestamp = since.timestamp();
        let mut statistics = AHashMap::new();
        for TwoTuple(_, fields) in entries.into_iter() {
            let entry = StreamEntry::try_from(fields)?;
            if entry.timestamp >= since_timestamp {
                let (n_wins, n_battles) = statistics.remove(&entry.tank_id).unwrap_or((0, 0));
                statistics.insert(
                    entry.tank_id,
                    (n_wins + entry.n_wins, n_battles + entry.n_battles),
                );
            }
        }
        let win_rates = statistics
            .into_iter()
            .map(|(tank_id, (n_wins, n_battles))| (tank_id, n_wins as f64 / n_battles as f64))
            .collect::<AHashMap<TankId, f64>>();
        let n_vehicles = win_rates.len();
        *self.win_rates.write().await = win_rates;

        tracing::info!(elapsed = %format_elapsed(&start_instant), n_vehicles = n_vehicles, "refreshed");
        Ok(())
    }
}
