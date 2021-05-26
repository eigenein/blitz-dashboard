use chrono::Duration;
use serde::Deserialize;

pub fn deserialize_duration_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Duration::seconds(i64::deserialize(deserializer)?))
}
