use chrono::{Duration, Utc};
use std::collections::hash_map::Entry;
use tokio::time::interval;
use tracing::{info, instrument};

use crate::aggregator::redis::store_vehicle_win_rates;
use crate::aggregator::stream::Stream;
use crate::aggregator::stream_entry::StreamEntry;
use crate::math::statistics::{ConfidenceInterval, Z};
use crate::opts::AggregateOpts;
use crate::wargaming::tank_id::TankId;
use crate::AHashMap;

pub mod redis;
pub mod stream;
pub mod stream_entry;

#[tracing::instrument(skip_all, fields(time_span = %opts.time_span))]
pub async fn run(opts: AggregateOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "aggregator"));

    let mut redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let mut stream = Stream::read(redis.clone(), opts.time_span).await?;
    let mut interval = interval(opts.interval);

    info!("running…");
    loop {
        interval.tick().await;
        stream.refresh().await?;

        let vehicle_win_rates = calculate_vehicle_win_rates(&stream.entries, opts.time_span);
        store_vehicle_win_rates(&mut redis, vehicle_win_rates).await?;
    }
}

#[instrument(level = "info", skip_all, fields(time_span = %time_span))]
fn calculate_vehicle_win_rates(
    sample: &[StreamEntry],
    time_span: Duration,
) -> AHashMap<TankId, ConfidenceInterval> {
    let mut statistics = AHashMap::default();
    let minimal_timestamp = (Utc::now() - time_span).timestamp();

    for point in sample {
        if point.timestamp >= minimal_timestamp {
            match statistics.entry(point.tank_id) {
                Entry::Vacant(entry) => {
                    entry.insert((point.n_battles, point.n_wins));
                }
                Entry::Occupied(mut entry) => {
                    let (n_battles, n_wins) = *entry.get();
                    entry.insert((n_battles + point.n_battles, n_wins + point.n_wins));
                }
            }
        }
    }

    statistics
        .into_iter()
        .map(|(tank_id, (n_battles, n_wins))| {
            (
                tank_id,
                ConfidenceInterval::wilson_score_interval(n_battles, n_wins, Z::default()),
            )
        })
        .collect()
}
