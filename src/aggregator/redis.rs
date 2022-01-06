use std::any::type_name;

use anyhow::Context;
use chrono::{Duration, Utc};
use redis::aio::MultiplexedConnection;
use redis::{from_redis_value, pipe, ErrorKind, FromRedisValue, RedisError, RedisResult, Value};
use tracing::instrument;

use crate::aggregator::stream_entry::{StreamEntry, StreamEntryBuilder};
use crate::math::statistics::ConfidenceInterval;
use crate::wargaming::tank_id::TankId;
use crate::AHashMap;

pub type Fields = KeyValueVec<String, i64>;
pub type Entry = TwoTuple<String, Fields>;
pub type StreamResponse = TwoTuple<(), Vec<Entry>>;
pub type XReadResponse = Vec<StreamResponse>;

pub const STREAM_KEY: &str = "streams::battles::v2";

const VEHICLE_WIN_RATES_KEY: &str = "vehicles::win_rates::ru";
const VEHICLE_WIN_RATE_MARGINS_KEY: &str = "vehicles::win_rates::margins::ru";

const TANK_ID_KEY: &str = "t";
const TIMESTAMP_KEY: &str = "ts";
const N_BATTLES_KEY: &str = "b";
const N_WINS_KEY: &str = "w";

#[instrument(level = "debug", skip_all, fields(n_entries = entries.len()))]
pub async fn push_entries(
    redis: &mut MultiplexedConnection,
    entries: &[StreamEntry],
    stream_duration: Duration,
) -> crate::Result {
    if entries.is_empty() {
        return Ok(());
    }

    let mut pipeline = pipe();

    for entry in entries.iter() {
        let mut items = vec![
            (TANK_ID_KEY, entry.tank_id as i64),
            (TIMESTAMP_KEY, entry.timestamp),
        ];
        if entry.n_battles != 1 {
            items.push((N_BATTLES_KEY, entry.n_battles as i64));
        }
        if entry.n_wins != 0 {
            items.push((N_WINS_KEY, entry.n_wins as i64));
        }
        pipeline.xadd(STREAM_KEY, "*", &items).ignore();
    }

    let minimum_id = (Utc::now() - stream_duration).timestamp_millis();
    tracing::debug!(minimum_id = minimum_id, "adding the stream entries…");
    pipeline
        .cmd("XTRIM")
        .arg(STREAM_KEY)
        .arg("MINID")
        .arg("~")
        .arg(minimum_id)
        .ignore();
    pipeline
        .query_async(redis)
        .await
        .context("failed to add the sample points to the stream")
}

#[instrument(skip_all, fields(n_vehicles = win_rates.len()))]
pub async fn store_vehicle_win_rates(
    redis: &mut MultiplexedConnection,
    win_rates: AHashMap<TankId, ConfidenceInterval>,
) -> crate::Result {
    let mut pipeline = pipe();

    pipeline.cmd("HMSET");
    pipeline.arg(VEHICLE_WIN_RATES_KEY);
    for (tank_id, win_rate) in win_rates.iter() {
        pipeline.arg(tank_id).arg(win_rate.mean);
    }
    pipeline.ignore();

    pipeline.cmd("HMSET");
    pipeline.arg(VEHICLE_WIN_RATE_MARGINS_KEY);
    for (tank_id, win_rate) in win_rates.into_iter() {
        pipeline.arg(tank_id).arg(win_rate.margin);
    }
    pipeline.ignore();

    pipeline
        .query_async(redis)
        .await
        .context("failed to store the vehicle win rates")
}

#[instrument(level = "debug", skip_all)]
pub async fn retrieve_vehicle_win_rates(
    redis: &mut MultiplexedConnection,
) -> crate::Result<AHashMap<TankId, ConfidenceInterval>> {
    let (means, mut margins): (AHashMap<TankId, f64>, AHashMap<TankId, f64>) = pipe()
        .hgetall(VEHICLE_WIN_RATES_KEY)
        .hgetall(VEHICLE_WIN_RATE_MARGINS_KEY)
        .query_async(redis)
        .await
        .context("failed to retrieve vehicle win rates")?;
    let win_rates = means
        .into_iter()
        .filter_map(|(tank_id, mean)| {
            margins
                .remove(&tank_id)
                .map(|margin| (tank_id, mean, margin))
        })
        .map(|(tank_id, mean, margin)| (tank_id, ConfidenceInterval { mean, margin }))
        .collect();

    Ok(win_rates)
}

impl TryFrom<KeyValueVec<String, i64>> for StreamEntry {
    type Error = anyhow::Error;

    fn try_from(map: KeyValueVec<String, i64>) -> crate::Result<Self> {
        let mut builder = StreamEntryBuilder::default();
        for (key, value) in map.0.into_iter() {
            match key.as_str() {
                "timestamp" | TIMESTAMP_KEY => {
                    builder.timestamp(value);
                }
                "tank_id" | TANK_ID_KEY => {
                    builder.tank_id(value.try_into()?);
                }
                "n_battles" | N_BATTLES_KEY => {
                    builder.n_battles(value.try_into()?);
                }
                "n_wins" | N_WINS_KEY => {
                    builder.n_wins(value.try_into()?);
                }
                "is_win" => {
                    builder.n_wins(value.try_into()?);
                }
                _ => {}
            }
        }
        builder.build()
    }
}

pub struct KeyValueVec<K, V>(pub Vec<(K, V)>);

impl<K: FromRedisValue, V: FromRedisValue> FromRedisValue for KeyValueVec<K, V> {
    #[tracing::instrument(skip_all)]
    fn from_redis_value(value: &Value) -> RedisResult<Self> {
        let inner = value
            .as_map_iter()
            .ok_or_else(|| {
                RedisError::from((
                    ErrorKind::TypeError,
                    "Response was of incompatible type",
                    format!("{:?} (response was {:?})", "Not hashmap compatible", value),
                ))
            })?
            .map(|(key, value)| Ok((from_redis_value(key)?, from_redis_value(value)?)))
            .collect::<RedisResult<Vec<(K, V)>>>()?;
        tracing::trace!(n_items = inner.len(), type_ = type_name::<Self>());
        Ok(Self(inner))
    }
}

/// Work around the bug in the `redis` crate.
/// https://github.com/mitsuhiko/redis-rs/issues/334
pub struct TwoTuple<T1, T2>(pub T1, pub T2);

impl<T1: FromRedisValue, T2: FromRedisValue> FromRedisValue for TwoTuple<T1, T2> {
    fn from_redis_value(value: &Value) -> RedisResult<Self> {
        match value {
            Value::Bulk(entries) if entries.len() == 2 => Ok(Self(
                from_redis_value(&entries[0])?,
                from_redis_value(&entries[1])?,
            )),
            _ => Err(RedisError::from((
                ErrorKind::TypeError,
                "Response was of incompatible type",
                format!("{:?} (response was {:?})", "Not a 2-tuple", value),
            ))),
        }
    }
}
