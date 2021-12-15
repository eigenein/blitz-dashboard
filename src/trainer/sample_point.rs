use chrono::Utc;
use redis::{ErrorKind, FromRedisValue, RedisError, RedisResult, Value};
use serde::{Deserialize, Serialize};

use crate::DateTime;

/// Single sample point of a dataset.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SamplePoint {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_test: bool,
    pub n_battles: i32,
    pub n_wins: i32,

    #[serde(default = "Utc::now")]
    pub timestamp: DateTime,
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
