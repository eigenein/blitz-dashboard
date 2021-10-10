//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::result::Result as StdResult;
use std::time::Instant;

use anyhow::Context;
use bytes::Bytes;
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands, Pipeline};
use serde::{Deserialize, Serialize};

use math::{adjust_factors, initialize_factors, predict_win_rate};

use crate::opts::TrainerOpts;
use crate::trainer::vector::Vector;
use crate::StdDuration;

pub mod math;
pub mod vector;

pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let mut redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;

    let account_ttl_secs: usize = opts.account_ttl.as_secs().try_into()?;
    let mut vehicle_factors_cache = HashMap::new();

    log::info!("Running…");
    loop {
        let start_instant = Instant::now();

        let mut total_error = 0.0;

        let mut account_factors_cache = HashMap::new();
        let mut n_new_accounts = 0;
        let mut n_initialized_accounts = 0;

        for _ in 0..opts.batch_size {
            let TrainStep {
                account_id,
                tank_id,
                is_win,
            } = get_random_step(&mut redis).await?;

            let account_factors = match account_factors_cache.entry(account_id) {
                Entry::Occupied(entry) => entry.into_mut(),

                Entry::Vacant(entry) => {
                    let mut factors = get_account_factors(&mut redis, account_id)
                        .await?
                        .unwrap_or_else(|| {
                            n_new_accounts += 1;
                            Vector::new()
                        });
                    if initialize_factors(&mut factors, opts.n_factors, opts.factor_std) {
                        n_initialized_accounts += 1;
                    }
                    entry.insert(factors)
                }
            };

            let vehicle_factors = match vehicle_factors_cache.entry(tank_id) {
                Entry::Occupied(entry) => entry.into_mut(),

                Entry::Vacant(entry) => {
                    let mut factors = get_vehicle_factors(&mut redis, tank_id)
                        .await?
                        .unwrap_or_else(Vector::new);
                    initialize_factors(&mut factors, opts.n_factors, opts.factor_std);
                    entry.insert(factors)
                }
            };

            let prediction = predict_win_rate(vehicle_factors, account_factors);
            let target = if is_win { 1.0 } else { 0.0 };
            let residual_error = target - prediction;

            let old_account_factors = account_factors.clone();
            adjust_factors(
                account_factors,
                vehicle_factors,
                residual_error,
                opts.account_learning_rate,
                opts.regularization,
            );
            adjust_factors(
                vehicle_factors,
                &old_account_factors,
                residual_error,
                opts.vehicle_learning_rate,
                opts.regularization,
            );

            if let Some(duplicate_id) = REMAP_TANK_ID.get(&tank_id) {
                let vehicle_factors = vehicle_factors.clone();
                vehicle_factors_cache.insert(*duplicate_id, vehicle_factors);
            }

            total_error -= residual_error;
        }

        set_all_accounts_factors(&mut redis, account_factors_cache, account_ttl_secs).await?;
        set_all_vehicles_factors(&mut redis, &vehicle_factors_cache).await?;

        let error = 100.0 * total_error / opts.batch_size as f64;
        let ewma = update_error_ewma(&mut redis, error, opts.ewma_factor).await?;
        log::info!(
            "AE: {:>+7.3} pp | EWMA: {:>+7.3} pp | {:>4.0} steps/s | init: {:>5} | new: {:>5}",
            error,
            ewma,
            opts.batch_size as f64 / start_instant.elapsed().as_secs_f64(),
            n_initialized_accounts,
            n_new_accounts,
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
        .ignore()
        .ltrim(TRAINER_QUEUE_KEY, -limit, -1)
        .ignore()
        .query_async(redis)
        .await
        .context("failed to push the steps")?;
    Ok(())
}

const TRAINER_QUEUE_KEY: &str = "trainer::steps";

async fn get_random_step(redis: &mut MultiplexedConnection) -> crate::Result<TrainStep> {
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

async fn set_all_vehicles_factors(
    redis: &mut MultiplexedConnection,
    vehicles_factors: &HashMap<i32, Vector>,
) -> crate::Result {
    let items: crate::Result<Vec<(i32, Vec<u8>)>> = vehicles_factors
        .iter()
        .map(|(tank_id, factors)| Ok((*tank_id, rmp_serde::to_vec(factors)?)))
        .collect();
    redis.hset_multiple(VEHICLE_FACTORS_KEY, &items?).await?;
    Ok(())
}

async fn get_account_factors(
    redis: &mut MultiplexedConnection,
    account_id: i32,
) -> crate::Result<Option<Vector>> {
    let bytes: Option<Vec<u8>> = redis.get(format!("f::ru::{}", account_id)).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

fn set_account_factors(
    pipeline: &mut Pipeline,
    account_id: i32,
    factors: &Vector,
    ttl_secs: usize,
) -> crate::Result {
    let bytes = rmp_serde::to_vec(factors)?;
    pipeline
        .set_ex(format!("f::ru::{}", account_id), bytes, ttl_secs)
        .ignore();
    Ok(())
}

async fn set_all_accounts_factors(
    redis: &mut MultiplexedConnection,
    accounts: HashMap<i32, Vector>,
    ttl_secs: usize,
) -> crate::Result {
    let mut pipeline = pipe();
    for (account_id, factors) in accounts.into_iter() {
        set_account_factors(&mut pipeline, account_id, &factors, ttl_secs)?;
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to update the accounts factors")
}

async fn update_error_ewma(
    redis: &mut MultiplexedConnection,
    error: f64,
    smoothing: f64,
) -> crate::Result<f64> {
    const KEY: &str = "trainer::error_ewma";
    let ewma: Option<f64> = redis.get(KEY).await?;
    let ewma = error * smoothing + ewma.unwrap_or(0.0) * (1.0 - smoothing);
    redis.set(KEY, ewma).await?;
    Ok(ewma)
}
