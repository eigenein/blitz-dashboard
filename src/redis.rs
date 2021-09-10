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

pub async fn get_vehicle_factors(redis: &mut Connection, tank_id: i32) -> crate::Result<Vec<f64>> {
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
    redis
        .hset(VEHICLE_FACTORS_KEY, tank_id, rmp_serde::to_vec(factors)?)
        .await?;
    Ok(())
}