mod models;
pub mod persistence;

use std::collections::hash_map::Entry;
use std::collections::VecDeque;

use chrono::{Duration, Utc};
use itertools::Itertools;
use redis::AsyncCommands;
use serde_json::json;
use tokio::time::interval;
use tracing::{info, instrument};

use crate::aggregator::models::{Analytics, DurationWrapper, Timeline, VehicleEntry};
use crate::aggregator::persistence::{store_analytics, store_charts, UPDATED_AT_KEY};
use crate::battle_stream::entry::DenormalizedStreamEntry;
use crate::battle_stream::stream::BattleStream;
use crate::math::statistics::{ConfidenceInterval, Z};
use crate::models::BattleCounts;
use crate::opts::AggregateOpts;
use crate::wargaming::tank_id::TankId;
use crate::{AHashMap, DateTime};

pub async fn run(opts: AggregateOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "aggregator"));

    let mut redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let mut stream = BattleStream::read(
        redis.clone(),
        *opts
            .time_spans
            .iter()
            .max()
            .unwrap()
            .max(&(opts.charts_time_span + opts.charts_window_span)),
    )
    .await?;
    let mut interval = interval(opts.interval);

    info!("running…");
    loop {
        interval.tick().await;
        stream.refresh().await?;
        let now = Utc::now();

        let analytics = calculate_analytics(&stream.entries, &opts.time_spans, now);
        store_analytics(&mut redis, &analytics).await?;

        let charts = build_timelines(
            &stream.entries,
            opts.charts_time_span,
            opts.charts_window_span,
            now,
        )
        .map(|(tank_id, timeline)| (tank_id, build_timeline_chart(tank_id, timeline)));
        store_charts(&mut redis, charts).await?;

        redis.set(UPDATED_AT_KEY, now.timestamp()).await?;
    }
}

#[instrument(level = "info", skip_all, fields(n_entries = entries.len()))]
fn calculate_analytics(
    entries: &[DenormalizedStreamEntry],
    time_spans: &[Duration],
    now: DateTime,
) -> Analytics {
    let deadlines = time_spans
        .iter()
        .map(|time_span| now - *time_span)
        .collect_vec();
    let mut statistics = AHashMap::default();

    for sample_entry in entries {
        match statistics.entry(sample_entry.tank.tank_id) {
            Entry::Vacant(entry) => {
                let value = deadlines
                    .iter()
                    .map(|deadline| {
                        if sample_entry.tank.timestamp > *deadline {
                            sample_entry.tank.battle_counts
                        } else {
                            BattleCounts::default()
                        }
                    })
                    .collect_vec();
                entry.insert(value);
            }

            Entry::Occupied(mut entry) => {
                for (value, deadline) in entry.get_mut().iter_mut().zip(&deadlines) {
                    if sample_entry.tank.timestamp > *deadline {
                        value.n_battles += sample_entry.tank.battle_counts.n_battles;
                        value.n_wins += sample_entry.tank.battle_counts.n_wins;
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
///
/// The entries MUST be sorted by timestamp.
#[instrument(
    skip_all,
    level = "info",
    fields(n_entries = entries.len(), window_span = window_span.to_string().as_str()),
)]
#[must_use]
fn build_timelines(
    entries: &[DenormalizedStreamEntry],
    time_span: Duration,
    window_span: Duration,
    now: DateTime,
) -> impl Iterator<Item = (TankId, Timeline)> {
    group_entries_by_tank_id(entries)
        .into_iter()
        .map(move |(tank_id, entries)| {
            (
                tank_id,
                build_vehicle_timeline(entries, time_span, window_span, now),
            )
        })
}

/// Groups the battle stream entries by tank ID.
#[instrument(skip_all)]
#[must_use]
fn group_entries_by_tank_id(
    entries: &[DenormalizedStreamEntry],
) -> AHashMap<TankId, Vec<VehicleEntry>> {
    let mut vehicle_entries = AHashMap::default();

    for stream_entry in entries {
        let vehicle_entry = VehicleEntry {
            timestamp: stream_entry.tank.timestamp,
            battle_counts: stream_entry.tank.battle_counts,
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

/// Builds the vehicle timeline.
///
/// The entries MUST be sorted by timestamp.
#[instrument(skip_all)]
#[must_use]
fn build_vehicle_timeline(
    entries: Vec<VehicleEntry>,
    time_span: Duration,
    window_span: Duration,
    now: DateTime,
) -> Timeline {
    let start_time = Utc::now() - time_span;
    let mut window = VecDeque::new();
    let mut battle_counts = BattleCounts::default();
    let mut timeline = Timeline::new();

    for entry in entries {
        let timestamp = entry.timestamp;
        cleanup_window(&mut window, &mut battle_counts, timestamp, window_span);

        battle_counts.n_battles += entry.battle_counts.n_battles;
        battle_counts.n_wins += entry.battle_counts.n_wins;
        window.push_back(entry);

        if timestamp >= start_time {
            timeline.push((
                timestamp,
                ConfidenceInterval::wilson_score_interval(
                    battle_counts.n_battles,
                    battle_counts.n_wins,
                    Z::default(),
                ),
            ));
        }
    }

    // Add the «now» entry to account for the latest trend.
    cleanup_window(&mut window, &mut battle_counts, now, window_span);
    timeline.push((
        now,
        ConfidenceInterval::wilson_score_interval(
            battle_counts.n_battles,
            battle_counts.n_wins,
            Z::default(),
        ),
    ));

    timeline
}

/// Removes the «expired» front entries from the window
/// and decreases the respective battle counts.
#[instrument(skip_all)]
fn cleanup_window(
    window: &mut VecDeque<VehicleEntry>,
    battle_counts: &mut BattleCounts,
    window_ends_at: DateTime,
    window_span: Duration,
) {
    while match window.front() {
        Some(first) if window_ends_at - first.timestamp >= window_span => {
            battle_counts.n_battles -= first.battle_counts.n_battles;
            battle_counts.n_wins -= first.battle_counts.n_wins;
            window.pop_front();
            true
        }
        _ => false,
    } {}
}

#[instrument(skip_all, level = "debug", fields(tank_id = tank_id))]
fn build_timeline_chart(tank_id: TankId, timeline: Timeline) -> serde_json::Value {
    const TENSION: f32 = 0.1;

    json!({
        "type": "line",
        "options": {
            "maintainAspectRatio": false,
            "zone": "system",
            "colorMode": "auto",
            "scales": {"x": {"type": "time"}, "y": {"position": "right"}},
            "plugins": {
                "tooltip": {
                    "mode": "index",
                    "intersect": false,
                    "position": "average",
                },
            }
        },
        "data": {
            "labels": timeline.iter().map(|(timestamp, _)| timestamp.timestamp_millis()).collect_vec(),
            "datasets": [
                {
                    "label": "Средний",
                    "data": timeline.iter().map(|(_, interval)| interval.mean * 100.0).collect_vec(),
                    "fill": false,
                    "tension": TENSION,
                },
                {
                    "label": "Верхний CI 95%",
                    "data": timeline.iter().map(|(_, interval)| interval.upper() * 100.0).collect_vec(),
                    "fill": 0,
                    "borderColor": "transparent",
                    "tension": TENSION,
                    "pointRadius": 0,
                },
                {
                    "label": "Нижний CI 95%",
                    "data": timeline.iter().map(|(_, interval)| interval.lower() * 100.0).collect_vec(),
                    "fill": 0,
                    "borderColor": "transparent",
                    "tension": TENSION,
                    "pointRadius": 0,
                },
            ],
        }
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn cleanup_window_ok() {
        let mut window = VecDeque::from([
            VehicleEntry {
                timestamp: Utc.timestamp(1, 0),
                battle_counts: BattleCounts {
                    n_battles: 1,
                    n_wins: 1,
                },
            },
            VehicleEntry {
                timestamp: Utc.timestamp(2, 0),
                battle_counts: BattleCounts {
                    n_battles: 1,
                    n_wins: 2,
                },
            },
            VehicleEntry {
                timestamp: Utc.timestamp(2, 1),
                battle_counts: BattleCounts::default(),
            },
            VehicleEntry {
                timestamp: Utc.timestamp(3, 0),
                battle_counts: BattleCounts::default(),
            },
        ]);
        let mut battle_counts = BattleCounts {
            n_battles: 4,
            n_wins: 4,
        };
        cleanup_window(
            &mut window,
            &mut battle_counts,
            Utc.timestamp(4, 0),
            Duration::seconds(2),
        );

        assert_eq!(battle_counts.n_battles, 4 - 1 - 1);
        assert_eq!(battle_counts.n_wins, 4 - 1 - 2);

        assert_eq!(window.len(), 2);
        assert_eq!(window.get(0).unwrap().timestamp, Utc.timestamp(2, 1));
        assert_eq!(window.get(1).unwrap().timestamp, Utc.timestamp(3, 0));
    }
}
