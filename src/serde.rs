use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Deserialize;

pub fn deserialize_duration_seconds<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Duration, D::Error> {
    Ok(Duration::seconds(i64::deserialize(deserializer)?))
}

pub fn epoch() -> DateTime<Utc> {
    Utc.timestamp(0, 0)
}
