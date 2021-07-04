use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Deserialize;

pub fn deserialize_duration_seconds<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Duration, D::Error> {
    Ok(Duration::seconds(i64::deserialize(deserializer)?))
}

pub fn deserialize_optional_timestamp<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error> {
    Ok(Option::<i64>::deserialize(deserializer)?.map(|timestamp| Utc.timestamp(timestamp, 0)))
}
