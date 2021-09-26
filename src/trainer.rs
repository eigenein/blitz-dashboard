use std::collections::HashMap;
use std::result::Result as StdResult;

use anyhow::Context;
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands};
use serde::{Deserialize, Serialize};

use crate::opts::TrainerOpts;

pub async fn run(_opts: TrainerOpts) -> crate::Result {
    Ok(())
}

const TRAINER_QUEUE_KEY: &str = "trainer::steps";
const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";

/// Some vehicles are just copies of some other vehicles.
/// Remap them to improve the latent factors.
static REMAP_TANK_ID: phf::Map<i32, i32> = phf::phf_map! {
    64273_i32 => 55313, // 8,8 cm Pak 43 Jagdtiger
    64769_i32 => 9217, // ИС-6 Бесстрашный
    64801_i32 => 2849, // T34 Independence
};

#[derive(Serialize, Deserialize)]
pub struct TrainStep {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_win: bool,
}

pub async fn get_vehicle_factors(
    redis: &mut MultiplexedConnection,
    tank_id: i32,
) -> crate::Result<Vec<f64>> {
    let tank_id = REMAP_TANK_ID.get(&tank_id).copied().unwrap_or(tank_id);
    let value: Option<Vec<u8>> = redis.hget(VEHICLE_FACTORS_KEY, tank_id).await?;
    match value {
        Some(value) => Ok(rmp_serde::from_read_ref(&value)?),
        None => Ok(Vec::new()),
    }
}

pub async fn get_all_vehicle_factors(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<i32, Vec<f64>>> {
    let hash_map: HashMap<i32, Vec<u8>> = redis.hgetall(VEHICLE_FACTORS_KEY).await?;
    hash_map
        .into_iter()
        .map(|(tank_id, value)| Ok((tank_id, rmp_serde::from_read_ref(&value)?)))
        .collect()
}

pub async fn set_vehicle_factors(
    redis: &mut MultiplexedConnection,
    tank_id: i32,
    factors: &[f64],
) -> crate::Result {
    let bytes = rmp_serde::to_vec(factors)?;
    let mut pipeline = pipe();
    pipeline.hset(VEHICLE_FACTORS_KEY, tank_id, &bytes);
    if let Some(tank_id) = REMAP_TANK_ID.get(&tank_id) {
        pipeline.hset(VEHICLE_FACTORS_KEY, *tank_id, bytes);
    }
    pipeline.query_async(redis).await?;
    Ok(())
}

pub async fn push_train_steps(
    redis: &mut MultiplexedConnection,
    steps: &[TrainStep],
    limit: isize,
) -> crate::Result {
    let serialized_steps: StdResult<Vec<Vec<u8>>, rmp_serde::encode::Error> =
        steps.iter().map(rmp_serde::to_vec).collect();
    let serialized_steps = serialized_steps.context("failed to serialize the steps")?;
    pipe()
        .lpush(TRAINER_QUEUE_KEY, serialized_steps)
        .ltrim(TRAINER_QUEUE_KEY, -limit, -1)
        .query_async(redis)
        .await
        .context("failed to push the steps")?;
    Ok(())
}
