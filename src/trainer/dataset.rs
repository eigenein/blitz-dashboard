use std::any::type_name;

use anyhow::{anyhow, Context};
use chrono::{Duration, TimeZone, Utc};
use humantime::format_duration;
use redis::aio::MultiplexedConnection;
use redis::streams::{StreamMaxlen, StreamReadOptions};
use redis::{
    from_redis_value, pipe, AsyncCommands, ErrorKind, FromRedisValue, RedisError, RedisResult,
    Value,
};
use tracing::instrument;

use crate::trainer::loss::BCELoss;
use crate::trainer::sample_point::SamplePoint;
use crate::trainer::stream_entry::{StreamEntry, StreamEntryBuilder};

const STREAM_KEY: &str = "streams::battles::v2";
const PAGE_SIZE: usize = 100000;
const ACCOUNT_ID_KEY: &str = "a";
const TANK_ID_KEY: &str = "t";
const TIMESTAMP_KEY: &str = "ts";
const N_BATTLES_KEY: &str = "b";
const N_WINS_KEY: &str = "w";
const IS_TEST_KEY: &str = "tt";

#[tracing::instrument(level = "debug", skip_all, fields(n_entries = entries.len()))]
pub async fn push_stream_entries(
    redis: &mut MultiplexedConnection,
    entries: &[StreamEntry],
    stream_size: usize,
    stream_duration: Duration,
) -> crate::Result {
    if entries.is_empty() {
        return Ok(());
    }

    let mut pipeline = pipe();
    let maxlen = StreamMaxlen::Approx(stream_size);

    for entry in entries.iter() {
        let mut items = vec![
            (ACCOUNT_ID_KEY, entry.account_id as i64),
            (TANK_ID_KEY, entry.tank_id as i64),
            (TIMESTAMP_KEY, entry.timestamp),
        ];
        if entry.n_battles != 1 {
            items.push((N_BATTLES_KEY, entry.n_battles as i64));
        }
        if entry.n_wins != 0 {
            items.push((N_WINS_KEY, entry.n_wins as i64));
        }
        if entry.is_test {
            items.push((IS_TEST_KEY, 1));
        }
        pipeline
            .xadd_maxlen(STREAM_KEY, maxlen, "*", &items)
            .ignore();
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

#[derive(Clone)]
pub struct Dataset {
    pub sample: Vec<SamplePoint>,
    pub baseline_loss: f64,
    pub redis: MultiplexedConnection,

    /// Last read entry ID of the Redis stream.
    pointer: String,

    time_span: Duration,
}

impl Dataset {
    #[instrument(skip_all, fields(time_span = %format_duration(time_span.to_std()?)))]
    pub async fn load(
        mut redis: MultiplexedConnection,
        time_span: Duration,
    ) -> crate::Result<Self> {
        let (pointer, sample) = load_sample(&mut redis, time_span).await?;
        let baseline_loss = calculate_baseline_loss(&sample);
        let first_timestamp = sample
            .first()
            .map(|point| point.timestamp)
            .unwrap_or_default();
        tracing::info!(
            first_timestamp = Utc.timestamp(first_timestamp, 0).to_string().as_str(),
            n_points = sample.len(),
            pointer = pointer.as_str(),
            baseline_loss = baseline_loss,
            "loaded",
        );
        Ok(Self {
            redis,
            sample,
            pointer,
            baseline_loss,
            time_span,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn refresh(&mut self) -> crate::Result {
        if let Some((_, new_pointer)) =
            refresh_sample(&mut self.redis, &self.pointer, &mut self.sample).await?
        {
            expire(&mut self.sample, self.time_span);
            self.pointer = new_pointer;
        }
        Ok(())
    }
}

/// Calculate the loss on the constant model that always predicts `0.5`.
#[tracing::instrument(skip_all)]
fn calculate_baseline_loss(sample: &[SamplePoint]) -> f64 {
    let mut loss = BCELoss::default();
    for point in sample {
        if point.is_test {
            loss.push_sample(0.5, point.is_win);
        }
    }
    loss.finalise()
}

/// Load sample points from the stream within the specified time span.
#[instrument(skip_all, fields(time_span = %format_duration(time_span.to_std()?)))]
async fn load_sample(
    redis: &mut MultiplexedConnection,
    time_span: Duration,
) -> crate::Result<(String, Vec<SamplePoint>)> {
    let mut sample = Vec::new();
    let mut pointer = "0".to_string();

    while match refresh_sample(redis, &pointer, &mut sample).await? {
        Some((n_entries, new_pointer)) => {
            let last_timestamp = sample
                .last()
                .map(|point| point.timestamp)
                .unwrap_or_default();
            tracing::info!(
                n_entries_read = n_entries,
                n_points_total = sample.len(),
                pointer = new_pointer.as_str(),
                last_timestamp = Utc.timestamp(last_timestamp, 0).to_string().as_str(),
                "loading…",
            );
            pointer = new_pointer;
            n_entries >= PAGE_SIZE
        }
        None => false,
    } {}

    tracing::info!(
        time_span = %format_duration(time_span.to_std()?),
        "removing expired points…",
    );
    expire(&mut sample, time_span);

    match sample.is_empty() {
        false => Ok((pointer, sample)),
        true => Err(anyhow!("training set is empty, try a longer time span")),
    }
}

/// Remove outdated sample points and append new ones.
#[tracing::instrument(level = "debug", skip(redis, sample))]
async fn refresh_sample(
    redis: &mut MultiplexedConnection,
    last_id: &str,
    sample: &mut Vec<SamplePoint>,
) -> crate::Result<Option<(usize, String)>> {
    // Fetch new points.
    type Fields = KeyValueVec<String, i64>;
    type Entry = TwoTuple<String, Fields>;
    type StreamResponse = TwoTuple<(), Vec<Entry>>;
    type XReadResponse = Vec<StreamResponse>;
    let mut response: XReadResponse = redis
        .xread_options(
            &[STREAM_KEY],
            &[&last_id],
            &StreamReadOptions::default().count(PAGE_SIZE),
        )
        .await?;
    match response.pop() {
        Some(TwoTuple(_, entries)) => {
            let result = entries
                .last()
                .map(|TwoTuple(id, _)| (entries.len(), id.clone()));
            for TwoTuple(_, entry) in entries.into_iter() {
                let points: Vec<SamplePoint> = StreamEntry::try_from(entry)?.into();
                sample.extend(points.into_iter());
            }
            Ok(result)
        }
        None => Ok(None),
    }
}

/// Removes expired sample points.
#[tracing::instrument(level = "debug", skip_all)]
fn expire(sample: &mut Vec<SamplePoint>, time_span: Duration) {
    let expiry_timestamp = (Utc::now() - time_span).timestamp();
    sample.retain(|point| point.timestamp > expiry_timestamp);
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
                "account_id" | ACCOUNT_ID_KEY => {
                    builder.account_id(value.try_into()?);
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
                "is_test" | IS_TEST_KEY if value == 1 => {
                    builder.set_test(true);
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

struct KeyValueVec<K, V>(pub Vec<(K, V)>);

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
        tracing::debug!(n_items = inner.len(), type_ = type_name::<Self>());
        Ok(Self(inner))
    }
}

/// Work around the bug in the `redis` crate.
/// https://github.com/mitsuhiko/redis-rs/issues/334
struct TwoTuple<T1, T2>(pub T1, pub T2);

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
