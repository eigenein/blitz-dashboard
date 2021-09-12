use std::collections::HashMap;

use anyhow::Context;
use redis::aio::ConnectionManager as Connection;
use redis::AsyncCommands;

pub async fn open(uri: &str) -> crate::Result<Connection> {
    Ok(redis::Client::open(uri)
        .context("failed to parse Redis URI")?
        .get_tokio_connection_manager()
        .await
        .context("failed to connect to Redis")?)
}

const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";
const GLOBAL_BIAS_KEY: &str = "cf::global_bias";

pub async fn get_vehicle_factors(redis: &mut Connection, tank_id: i32) -> crate::Result<Vec<f64>> {
    let tank_id = match tank_id {
        64273 => 55313, // 8,8 cm Pak 43 Jagdtiger
        _ => tank_id,
    };
    let value: Option<Vec<u8>> = redis.hget(VEHICLE_FACTORS_KEY, tank_id).await?;
    match value {
        Some(value) => Ok(rmp_serde::from_read_ref(&value)?),
        None => Ok(Vec::new()),
    }
}

pub async fn get_all_vehicle_factors(
    redis: &mut Connection,
) -> crate::Result<HashMap<i32, Vec<f64>>> {
    let hash_map: HashMap<i32, Vec<u8>> = redis.hgetall(VEHICLE_FACTORS_KEY).await?;
    hash_map
        .into_iter()
        .map(|(tank_id, value)| Ok((tank_id, rmp_serde::from_read_ref(&value)?)))
        .collect()
}

pub async fn set_vehicle_factors(
    redis: &mut Connection,
    tank_id: i32,
    factors: &[f64],
) -> crate::Result {
    let bytes = rmp_serde::to_vec(factors)?;
    let mut pipeline = ::redis::pipe();
    pipeline.hset(VEHICLE_FACTORS_KEY, tank_id, &bytes);
    if tank_id == 55313 {
        // 8,8 cm Pak 43 Jagdtiger
        pipeline.hset(VEHICLE_FACTORS_KEY, 64273, bytes);
    }
    pipeline.query_async(redis).await?;
    Ok(())
}

pub async fn get_global_bias(redis: &mut Connection) -> crate::Result<f64> {
    let value: Option<f64> = redis.get(GLOBAL_BIAS_KEY).await?;
    Ok(value.unwrap_or(0.5))
}

pub async fn set_global_bias(redis: &mut Connection, value: f64) -> crate::Result {
    redis.set(GLOBAL_BIAS_KEY, value).await?;
    Ok(())
}
