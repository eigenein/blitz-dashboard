use std::str::FromStr;

use anyhow::anyhow;
use chrono::{Duration, TimeZone, Utc};
use redis::aio::MultiplexedConnection;
use redis::streams::StreamReadOptions;
use redis::{AsyncCommands, Value};

use crate::helpers::format_duration;
use crate::trainer::loss::BCELoss;
use crate::trainer::sample_point::SamplePoint;
use crate::DateTime;

pub const TRAIN_STREAM_KEY: &str = "streams::battles";
const REFRESH_POINTS_LIMIT: usize = 100000;

#[derive(Clone)]
pub struct Dataset {
    pub sample: Vec<(DateTime, SamplePoint)>,
    pub baseline_loss: f64,
    pub redis: Option<MultiplexedConnection>,

    /// Last read entry ID of the Redis stream.
    pointer: String,

    time_span: Duration,
}

impl Dataset {
    #[tracing::instrument(
        skip_all,
        fields(time_span = format_duration(time_span.to_std()?).as_str(), is_online = is_online),
    )]
    pub async fn load(
        mut redis: MultiplexedConnection,
        time_span: Duration,
        is_online: bool,
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
            redis: if is_online { Some(redis) } else { None },
            sample,
            pointer,
            baseline_loss,
            time_span,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn refresh(&mut self) -> crate::Result {
        if let Some(redis) = &mut self.redis {
            if let Some((_, new_pointer)) =
                refresh_sample(redis, &self.pointer, &mut self.sample, self.time_span).await?
            {
                self.pointer = new_pointer;
            }
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
    loss.average()
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
            n_points >= REFRESH_POINTS_LIMIT
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
    let options = StreamReadOptions::default().count(REFRESH_POINTS_LIMIT);
    let reply: Value = redis
        .xread_options(&[TRAIN_STREAM_KEY], &[&last_id], &options)
        .await?;
    let entries = parse_multiple_streams(reply)?;
    let result = entries.last().map(|(id, _)| (entries.len(), id.clone()));
    for (id, battle) in entries {
        sample.push((parse_entry_id(&id)?, battle));
    }
    tracing::debug!(n_points = result.as_ref().map(|result| result.0));
    Ok(result)
}

/// Parse Redis stream entry ID.
fn parse_entry_id(id: &str) -> crate::Result<DateTime> {
    let millis = id
        .split_once("-")
        .ok_or_else(|| anyhow!("unexpected stream entry ID"))?
        .0;
    Ok(Utc.timestamp_millis(i64::from_str(millis)?))
}

fn parse_multiple_streams(reply: Value) -> crate::Result<Vec<(String, SamplePoint)>> {
    match reply {
        Value::Nil => Ok(Vec::new()),
        Value::Bulk(mut streams) => match streams.pop() {
            Some(Value::Bulk(mut stream)) => match stream.pop() {
                Some(value) => parse_stream(value),
                other => Err(anyhow!("expected entries, got: {:?}", other)),
            },
            other => Err(anyhow!("expected (name, entries), got: {:?}", other)),
        },
        other => Err(anyhow!("expected a bulk of streams, got: {:?}", other)),
    }
}

fn parse_stream(reply: Value) -> crate::Result<Vec<(String, SamplePoint)>> {
    match reply {
        Value::Nil => Ok(Vec::new()),
        Value::Bulk(entries) => entries.into_iter().map(parse_stream_entry).collect(),
        other => Err(anyhow!("expected a bulk of entries, got: {:?}", other)),
    }
}

fn parse_stream_entry(reply: Value) -> crate::Result<(String, SamplePoint)> {
    match reply {
        Value::Bulk(mut entry) => {
            let fields = entry.pop();
            let id = entry.pop();
            match (id, fields) {
                (Some(Value::Data(id)), Some(Value::Bulk(mut fields))) => {
                    let value = fields.pop();
                    match value {
                        Some(Value::Data(data)) => {
                            Ok((String::from_utf8(id)?, rmp_serde::from_read_ref(&data)?))
                        }
                        other => Err(anyhow!("expected a binary data, got: {:?}", other)),
                    }
                }
                other => Err(anyhow!("expected (ID, fields), got: {:?}", other)),
            }
        }
        other => Err(anyhow!("expected (ID, fields), got: {:?}", other)),
    }
}
