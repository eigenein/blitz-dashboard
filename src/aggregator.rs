mod models;
pub mod persistence;

use std::collections::hash_map::Entry;

use chrono::{Duration, TimeZone, Utc};
use itertools::Itertools;
use redis::AsyncCommands;
use tokio::time::interval;
use tracing::{info, instrument};

use crate::aggregator::models::{Analytics, BattleCounts, DurationWrapper, VehicleEntry};
use crate::aggregator::persistence::{store_analytics, UPDATED_AT_KEY};
use crate::battle_stream::entry::DenormalizedStreamEntry;
use crate::battle_stream::stream::BattleStream;
use crate::math::statistics::{ConfidenceInterval, Z};
use crate::opts::AggregateOpts;
use crate::wargaming::tank_id::TankId;
use crate::AHashMap;

#[tracing::instrument(skip_all)]
pub async fn run(opts: AggregateOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "aggregator"));

    let mut redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let mut stream =
        BattleStream::read(redis.clone(), *opts.time_spans.iter().max().unwrap()).await?;
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
fn calculate_analytics(entries: &[DenormalizedStreamEntry], time_spans: &[Duration]) -> Analytics {
    let now = Utc::now();
    let deadlines = time_spans
        .iter()
        .map(|time_span| (now - *time_span).timestamp())
        .collect_vec();
    let mut statistics = AHashMap::default();

    for sample_entry in entries {
        match statistics.entry(sample_entry.tank.tank_id) {
            Entry::Vacant(entry) => {
                let value = deadlines
                    .iter()
                    .map(|deadline| {
                        if sample_entry.tank.timestamp >= *deadline {
                            BattleCounts {
                                n_battles: sample_entry.tank.n_battles,
                                n_wins: sample_entry.tank.n_wins,
                            }
                        } else {
                            BattleCounts::default()
                        }
                    })
                    .collect_vec();
                entry.insert(value);
            }

            Entry::Occupied(mut entry) => {
                for (value, deadline) in entry.get_mut().iter_mut().zip(&deadlines) {
                    if sample_entry.tank.timestamp >= *deadline {
                        value.n_battles += sample_entry.tank.n_battles;
                        value.n_wins += sample_entry.tank.n_wins;
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

/// For each vehicle in the stream builds the win-rate timeline.
#[instrument(skip_all)]
fn build_timelines(entries: &[DenormalizedStreamEntry]) -> Vec<(TankId, ())> {
    group_entries_by_tank_id(entries)
        .into_iter()
        .map(|(tank_id, entries)| (tank_id, build_vehicle_timeline(entries)))
        .collect()
}

/// Groups the battle stream entries by tank ID.
#[instrument(skip_all)]
fn group_entries_by_tank_id(
    entries: &[DenormalizedStreamEntry],
) -> AHashMap<TankId, Vec<VehicleEntry>> {
    let mut vehicle_entries = AHashMap::default();

    for stream_entry in entries {
        let vehicle_entry = VehicleEntry {
            timestamp: Utc.timestamp(stream_entry.tank.timestamp, 0),
            battle_counts: BattleCounts {
                n_battles: stream_entry.tank.n_battles,
                n_wins: stream_entry.tank.n_wins,
            },
        };
        match vehicle_entries.entry(stream_entry.tank.tank_id) {
            Entry::Vacant(entry) => {
                entry.insert(vec![vehicle_entry]);
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(vehicle_entry);
            }
        }
    }

    vehicle_entries
}

#[instrument(skip_all)]
fn build_vehicle_timeline(_entries: Vec<VehicleEntry>) {
    unimplemented!();
}
