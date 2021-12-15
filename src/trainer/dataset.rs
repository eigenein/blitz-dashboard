use std::str::FromStr;

use anyhow::{anyhow, Context};
use chrono::{Duration, TimeZone, Utc};
use redis::aio::MultiplexedConnection;
use redis::streams::{StreamMaxlen, StreamReadOptions};
use redis::{pipe, AsyncCommands, ErrorKind, FromRedisValue, RedisError, RedisResult, Value};

use crate::helpers::format_duration;
use crate::trainer::loss::BCELoss;
use crate::trainer::sample_point::SamplePoint;
use crate::{DateTime, StdResult};

const STREAM_KEY: &str = "streams::battles";
const STREAM_V2_KEY: &str = "streams::battles::v2";
const PAGE_SIZE: usize = 100000;

pub async fn push_sample_points(
    redis: &mut MultiplexedConnection,
    points: &[SamplePoint],
    stream_size: usize,
) -> crate::Result {
    let mut pipeline = pipe();
    let maxlen = StreamMaxlen::Approx(stream_size);

    for point in points.iter() {
        let items = &[
            ("account_id", point.account_id as i64),
            ("tank_id", point.tank_id as i64),
            ("n_battles", point.n_battles as i64),
            ("n_wins", point.n_wins as i64),
            ("timestamp", point.timestamp.timestamp()),
        ];
        pipeline
            .xadd_maxlen(STREAM_V2_KEY, maxlen, "*", items)
            .ignore();
    }

    // The following part is deprecated:
    let points: StdResult<Vec<Vec<u8>>, rmp_serde::encode::Error> =
        points.iter().map(rmp_serde::to_vec).collect();
    let points = points.context("failed to serialize the battles")?;
    for point in points {
        pipeline
            .xadd_maxlen(STREAM_KEY, maxlen, "*", &[("b", point)])
            .ignore();
    }

    pipeline
        .query_async(redis)
        .await
        .context("failed to add the sample points to the stream")
}

#[derive(Clone)]
pub struct Dataset {
    pub sample: Vec<(DateTime, SamplePoint)>,
    pub baseline_loss: f64,
    pub redis: MultiplexedConnection,

    /// Last read entry ID of the Redis stream.
    pointer: String,

    time_span: Duration,
}

impl Dataset {
    #[tracing::instrument(
        skip_all,
        fields(time_span = format_duration(time_span.to_std()?).as_str()),
    )]
    pub async fn load(
        mut redis: MultiplexedConnection,
        time_span: Duration,
    ) -> crate::Result<Self> {
        let (pointer, sample) = load_sample(&mut redis, time_span).await?;
        let baseline_loss = calculate_baseline_loss(&sample);
        tracing::info!(
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
        if let Some((_, new_pointer)) = refresh_sample(
            &mut self.redis,
            &self.pointer,
            &mut self.sample,
            self.time_span,
        )
        .await?
        {
            self.pointer = new_pointer;
        }
        Ok(())
    }
}

/// Calculate the loss on the constant model that always predicts `0.5`.
#[tracing::instrument(skip_all)]
fn calculate_baseline_loss(sample: &[(DateTime, SamplePoint)]) -> f64 {
    let mut loss = BCELoss::default();
    for (_, point) in sample {
        if point.is_test {
            loss.push_sample(0.5, point.n_wins as f64 / point.n_battles as f64);
        }
    }
    loss.finalise()
}

/// Load sample points from the stream within the specified time span.
#[tracing::instrument(skip_all, fields(time_span = format_duration(time_span.to_std()?).as_str()))]
async fn load_sample(
    redis: &mut MultiplexedConnection,
    time_span: Duration,
) -> crate::Result<(String, Vec<(DateTime, SamplePoint)>)> {
    let mut sample = Vec::new();
    let mut pointer = (Utc::now() - time_span).timestamp_millis().to_string();

    while match refresh_sample(redis, &pointer, &mut sample, time_span).await? {
        Some((n_points, new_pointer)) => {
            tracing::info!(n_points = sample.len(), pointer = new_pointer.as_str());
            pointer = new_pointer;
            n_points >= PAGE_SIZE
        }
        None => false,
    } {}

    match sample.is_empty() {
        false => Ok((pointer, sample)),
        true => Err(anyhow!("training set is empty, try a longer time span")),
    }
}

/// Remove outdated sample points and append new ones.
#[tracing::instrument(level = "debug", skip(redis, sample, time_span))]
async fn refresh_sample(
    redis: &mut MultiplexedConnection,
    last_id: &str,
    sample: &mut Vec<(DateTime, SamplePoint)>,
    time_span: Duration,
) -> crate::Result<Option<(usize, String)>> {
    // Remove the expired points.
    let expire_time = Utc::now() - time_span;
    sample.retain(|(timestamp, _)| timestamp > &expire_time);

    // Fetch new points.
    let options = StreamReadOptions::default().count(PAGE_SIZE);
    #[allow(clippy::type_complexity)]
    let mut reply: Vec<Option<((), Vec<Option<(String, SamplePoint)>>)>> = redis
        .xread_options(&[STREAM_KEY], &[&last_id], &options)
        .await?;
    let (_, entries) = reply
        .pop()
        .unwrap_or_else(|| Some(((), Vec::new())))
        .expect("wrapping `Option` is always `Some`");
    let result = entries
        .last()
        .map(|entry| entry.as_ref().unwrap())
        .map(|(id, _)| (entries.len(), id.clone()));
    for entry in entries.into_iter() {
        let (id, point) = entry.expect("wrapping `Option` is always `Some`");
        sample.push((parse_entry_id(&id)?, point));
    }
    tracing::debug!(n_points = result.as_ref().map(|result| result.0));
    Ok(result)
}

impl FromRedisValue for SamplePoint {
    fn from_redis_value(value: &Value) -> RedisResult<Self> {
        let mut map = value.as_map_iter().ok_or_else(|| {
            RedisError::from((
                ErrorKind::TypeError,
                "expected a map-compatible type",
                format!("{:?}", value),
            ))
        })?;
        let (_, value) = map.next().ok_or_else(|| {
            RedisError::from((
                ErrorKind::TypeError,
                "expected a non-empty map",
                format!("{:?}", value),
            ))
        })?;
        let value = if let Value::Data(value) = value {
            value
        } else {
            return Err(RedisError::from((
                ErrorKind::TypeError,
                "expected a binary data",
                format!("{:?}", value),
            )));
        };
        rmp_serde::from_read_ref(value).map_err(|error| {
            RedisError::from((
                ErrorKind::TypeError,
                "failed to deserialize",
                format!("{:?}", error),
            ))
        })
    }
}

/// Parse Redis stream entry ID.
fn parse_entry_id(id: &str) -> crate::Result<DateTime> {
    let millis = id
        .split_once("-")
        .ok_or_else(|| anyhow!("unexpected stream entry ID"))?
        .0;
    Ok(Utc.timestamp_millis(i64::from_str(millis)?))
}
