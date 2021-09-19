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

/// Some vehicles are just copies of some other vehicles.
/// Remap them to improve the latent factors.
static REMAP_TANK_ID: phf::Map<i32, i32> = phf::phf_map! {
    64273_i32 => 55313, // 8,8 cm Pak 43 Jagdtiger
    64801_i32 => 2849, // T34 Independence
};

pub async fn get_vehicle_factors(redis: &mut Connection, tank_id: i32) -> crate::Result<Vec<f64>> {
    let tank_id = REMAP_TANK_ID.get(&tank_id).copied().unwrap_or(tank_id);
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
    if let Some(tank_id) = REMAP_TANK_ID.get(&tank_id) {
        pipeline.hset(VEHICLE_FACTORS_KEY, *tank_id, bytes);
    }
    pipeline.query_async(redis).await?;
    Ok(())
}
