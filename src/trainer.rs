//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::result::Result as StdResult;
use std::time::Instant;

use anyhow::Context;
use bytes::Bytes;
use moka::future::CacheBuilder;
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands};
use serde::{Deserialize, Serialize};

use math::{adjust_factors, initialize_factors, predict_win_rate};

use crate::database::{open as open_database, retrieve_account_factors, update_account_factors};
use crate::opts::TrainerOpts;
use crate::trainer::vector::Vector;
use crate::StdDuration;

pub mod math;
pub mod vector;

pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let connections = &opts.connections;
    let database = open_database(&connections.database_uri, connections.initialize_schema).await?;
    let mut redis = redis::Client::open(connections.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;

    let account_factors_cache = CacheBuilder::new(opts.account_cache_size).build();
    let mut vehicle_factors_cache = HashMap::new();

    loop {
        let start_instant = Instant::now();

        let mut error = 0.0;
        let mut modified_tank_ids = HashSet::new();

        for _ in 0..opts.batch_size {
            let step = get_random_step(&mut redis).await?;

            let mut account_factors = match account_factors_cache.get(&step.account_id) {
                Some(factors) => factors,
                None => {
                    let mut factors = retrieve_account_factors(&database, step.account_id)
                        .await?
                        .unwrap_or_else(Vector::new);
                    initialize_factors(&mut factors, opts.n_factors);
                    factors
                }
            };

            if let Entry::Vacant(entry) = vehicle_factors_cache.entry(step.tank_id) {
                let mut factors = get_vehicle_factors(&mut redis, step.tank_id)
                    .await?
                    .unwrap_or_else(Vector::new);
                initialize_factors(&mut factors, opts.n_factors);
                entry.insert(factors);
            }
            let vehicle_factors = vehicle_factors_cache.get_mut(&step.tank_id).unwrap();

            let prediction = predict_win_rate(vehicle_factors, &account_factors);
            let target = if step.is_win { 1.0 } else { 0.0 };
            let residual_error = target - prediction;

            let cloned_account_factors = account_factors.clone();
            adjust_factors(
                &mut account_factors,
                vehicle_factors,
                residual_error,
                opts.account_learning_rate,
                opts.regularization,
            );
            adjust_factors(
                vehicle_factors,
                &cloned_account_factors,
                residual_error,
                opts.vehicle_learning_rate,
                opts.regularization,
            );

            error -= residual_error;
            modified_tank_ids.insert(step.tank_id);
            update_account_factors(&database, step.account_id, &account_factors).await?;
            account_factors_cache
                .insert(step.account_id, account_factors)
                .await;
        }

        set_vehicles_factors(&mut redis, &vehicle_factors_cache, modified_tank_ids).await?;

        log::info!(
            "Error: {:>7.3} pp | {:5.0} steps/s",
            100.0 * error / opts.batch_size as f64,
            opts.batch_size as f64 / start_instant.elapsed().as_secs_f64(),
        );
    }
}

#[derive(Serialize, Deserialize)]
pub struct TrainStep {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_win: bool,
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
        .rpush(TRAINER_QUEUE_KEY, serialized_steps)
        .ltrim(TRAINER_QUEUE_KEY, -limit, -1)
        .query_async(redis)
        .await
        .context("failed to push the steps")?;
    Ok(())
}

const TRAINER_QUEUE_KEY: &str = "trainer::steps";

pub async fn get_random_step(redis: &mut MultiplexedConnection) -> crate::Result<TrainStep> {
    loop {
        let queue_length = redis.llen(TRAINER_QUEUE_KEY).await?;
        if queue_length != 0 {
            let index = fastrand::isize(0..queue_length);
            let bytes: Bytes = redis.lindex(TRAINER_QUEUE_KEY, index).await?;
            break Ok(rmp_serde::from_read_ref(&bytes)?);
        }
        tokio::time::sleep(StdDuration::from_secs(1)).await;
    }
}

const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";

pub async fn get_vehicle_factors(
    redis: &mut MultiplexedConnection,
    tank_id: i32,
) -> crate::Result<Option<Vector>> {
    let bytes: Option<Bytes> = redis.hget(VEHICLE_FACTORS_KEY, tank_id).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

pub async fn get_all_vehicle_factors(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<i32, Vector>> {
    let hash_map: HashMap<i32, Vec<u8>> = redis.hgetall(VEHICLE_FACTORS_KEY).await?;
    hash_map
        .into_iter()
        .map(|(tank_id, value)| Ok((tank_id, rmp_serde::from_read_ref(&value)?)))
        .collect()
}

/// Some vehicles are just copies of some other vehicles.
/// Remap them to improve the latent factors.
static REMAP_TANK_ID: phf::Map<i32, i32> = phf::phf_map! {
    64273_i32 => 55313, // 8,8 cm Pak 43 Jagdtiger
    64769_i32 => 9217, // ИС-6 Бесстрашный
    64801_i32 => 2849, // T34 Independence
};

async fn set_vehicles_factors(
    redis: &mut MultiplexedConnection,
    vehicles_factors: &HashMap<i32, Vector>,
    tank_ids: HashSet<i32>,
) -> crate::Result {
    let mut pipeline = pipe();
    for tank_id in tank_ids.into_iter() {
        let bytes = rmp_serde::to_vec(&vehicles_factors[&tank_id])?;
        pipeline.hset(VEHICLE_FACTORS_KEY, tank_id, &bytes);
        if let Some(tank_copy_id) = REMAP_TANK_ID.get(&tank_id) {
            pipeline.hset(VEHICLE_FACTORS_KEY, *tank_copy_id, bytes);
        }
    }
    pipeline.query_async(redis).await?;
    Ok(())
}
