use chrono::Duration;
use serde::{Deserialize, Serializer};

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
