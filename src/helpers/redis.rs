use std::any::type_name;

use redis::{from_redis_value, ErrorKind, FromRedisValue, RedisError, RedisResult, Value};

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
