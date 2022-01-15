mod models;
pub mod persistence;

use std::collections::hash_map::Entry;

use chrono::{Duration, Utc};
use itertools::Itertools;
use redis::AsyncCommands;
use tokio::time::interval;
use tracing::{info, instrument};

use crate::aggregator::models::{Analytics, BattleCount, DurationWrapper};
use crate::aggregator::persistence::{store_analytics, UPDATED_AT_KEY};
use crate::battle_stream::entry::StreamEntry;
use crate::battle_stream::stream::Stream;
use crate::math::statistics::{ConfidenceInterval, Z};
use crate::opts::AggregateOpts;
use crate::AHashMap;

#[tracing::instrument(skip_all)]
pub async fn run(opts: AggregateOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "aggregator"));

    let mut redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let mut stream = Stream::read(redis.clone(), *opts.time_spans.iter().max().unwrap()).await?;
    let mut interval = interval(opts.interval);

    info!("running…");
    loop {
        interval.tick().await;
        stream.refresh().await?;

        let analytics = calculate_analytics(&stream.entries, &opts.time_spans);
        store_analytics(&mut redis, &analytics).await?;

        redis.set(UPDATED_AT_KEY, Utc::now().timestamp()).await?;
    }
}

#[instrument(level = "info", skip_all)]
fn calculate_analytics(sample: &[StreamEntry], time_spans: &[Duration]) -> Analytics {
    let now = Utc::now();
    let deadlines = time_spans
        .iter()
        .map(|time_span| (now - *time_span).timestamp())
        .collect_vec();
    let mut statistics = AHashMap::default();

    for point in sample {
        match statistics.entry(point.tank_id) {
            Entry::Vacant(entry) => {
                let value = deadlines
                    .iter()
                    .map(|deadline| {
                        if point.timestamp >= *deadline {
                            BattleCount {
                                n_battles: point.n_battles,
                                n_wins: point.n_wins,
                            }
                        } else {
                            BattleCount::default()
                        }
                    })
                    .collect_vec();
                entry.insert(value);
            }

            Entry::Occupied(mut entry) => {
                for (value, deadline) in entry.get_mut().iter_mut().zip(&deadlines) {
                    if point.timestamp >= *deadline {
                        value.n_battles += point.n_battles;
                        value.n_wins += point.n_wins;
                    }
                }
            }
        }
    }

    Analytics {
        time_spans: time_spans
            .iter()
            .map(|time_span| DurationWrapper {
                duration: *time_span,
            })
            .collect(),
        win_rates: statistics
            .into_iter()
            .map(|(tank_id, counts)| {
                (
                    tank_id,
                    counts
                        .into_iter()
                        .map(|count| {
                            if count.n_battles != 0 {
                                Some(ConfidenceInterval::wilson_score_interval(
                                    count.n_battles,
                                    count.n_wins,
                                    Z::default(),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}
