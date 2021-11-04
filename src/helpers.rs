use std::fmt::{Display, Formatter};
use std::time::Duration as StdDuration;

use anyhow::anyhow;
use chrono::Duration;
use humantime::format_duration;
use serde::{Deserialize, Serializer};
use tokio::task::spawn_blocking;

pub const fn from_minutes(minutes: u64) -> StdDuration {
    StdDuration::from_secs(minutes * 60)
}

pub const fn from_hours(hours: u64) -> StdDuration {
    from_minutes(hours * 60)
}

pub const fn from_days(days: u64) -> StdDuration {
    from_hours(days * 24)
}

pub const fn from_months(months: u64) -> StdDuration {
    StdDuration::from_secs(months * 2630016)
}

#[allow(dead_code)]
pub const fn from_years(years: u64) -> StdDuration {
    StdDuration::from_secs(years * 31557600)
}

pub struct Instant(std::time::Instant);

impl Instant {
    pub fn now() -> Self {
        Self(std::time::Instant::now())
    }

    pub fn elapsed(&self) -> Elapsed {
        Elapsed(self.0.elapsed())
    }
}

pub struct Elapsed(StdDuration);

impl Display for Elapsed {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&format_duration(self.0).to_string())
    }
}

pub async fn compress_to_vec(input: Vec<u8>, level: u8) -> crate::Result<Vec<u8>> {
    Ok(spawn_blocking(move || miniz_oxide::deflate::compress_to_vec(&input, level)).await?)
}

pub async fn decompress_to_vec(input: Vec<u8>) -> crate::Result<Vec<u8>> {
    spawn_blocking(move || miniz_oxide::inflate::decompress_to_vec(&input))
        .await?
        .map_err(|error| anyhow!("failed to decompress the input: {:?}", error))
}

pub fn serialize_duration_seconds<S: Serializer>(
    value: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(value.num_seconds())
}

pub fn deserialize_duration_seconds<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Duration, D::Error> {
    Ok(Duration::seconds(i64::deserialize(deserializer)?))
}
