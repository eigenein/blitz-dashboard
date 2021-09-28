//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::collections::HashMap;
use std::result::Result as StdResult;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use itertools::Itertools;
use log::Level;
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands};
use serde::{Deserialize, Serialize};

use cf::{adjust_factors, initialize_factors, predict_win_rate};

use crate::database::{open as open_database, retrieve_accounts_factors, update_account_factors};
use crate::metrics::Stopwatch;
use crate::opts::TrainerOpts;

pub mod cf;
pub mod vector;

pub async fn run(mut opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let connections = &opts.connections;
    let database = open_database(&connections.database_uri, connections.initialize_schema).await?;
    let mut redis = redis::Client::open(connections.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;

    let mut vehicles_factors = get_all_vehicle_factors(&mut redis).await?;
    log::info!("Running in batches of {} steps…", opts.batch_size);

    loop {
        reload_configuration(&mut redis, &mut opts).await?;
        let mut batch = get_batch(&mut redis, opts.batch_size).await?;
        let account_ids: Vec<i32> = batch.iter().map(|step| step.account_id).unique().collect();
        let tank_ids: Vec<i32> = batch.iter().map(|step| step.tank_id).unique().collect();
        fastrand::shuffle(&mut batch);

        reload_configuration(&mut redis, &mut opts).await?;
        let mut accounts_factors = retrieve_accounts_factors(&database, &account_ids).await?;
        for factors in accounts_factors.values_mut() {
            initialize_factors(factors, opts.n_factors);
        }

        let mut error = 0.0;
        for step in &batch {
            let account_factors = accounts_factors
                .get_mut(&step.account_id)
                .ok_or_else(|| anyhow!("no factors found for account #{}", step.account_id))?;
            let vehicle_factors =
                borrow_vehicle_factors(&mut vehicles_factors, step.tank_id, opts.n_factors)?;
            let prediction = predict_win_rate(vehicle_factors, account_factors);
            let target = if step.is_win { 1.0 } else { 0.0 };

            let residual_error = target - prediction;
            error -= residual_error;

            let frozen_account_factors = account_factors.to_vec();
            adjust_factors(
                account_factors,
                vehicle_factors,
                residual_error,
                opts.account_learning_rate,
                opts.regularization,
            );
            adjust_factors(
                vehicle_factors,
                &frozen_account_factors,
                residual_error,
                opts.vehicle_learning_rate,
                opts.regularization,
            );
        }

        log::debug!("Updating the vehicles factors…");
        set_vehicle_factors(&mut redis, &vehicles_factors, &tank_ids).await?;

        let n_accounts = accounts_factors.len();
        log::debug!("Updating factors for {} accounts…", n_accounts);
        for (account_id, factors) in accounts_factors.into_iter() {
            update_account_factors(&database, account_id, &factors).await?;
        }

        log::info!(
            "Error: {:>6.3} pp | accounts: {:>4} | vehicles: {:>3}",
            100.0 * error / opts.batch_size as f64,
            n_accounts,
            tank_ids.len(),
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

async fn reload_configuration(
    redis: &mut MultiplexedConnection,
    opts: &mut TrainerOpts,
) -> crate::Result {
    const N_FACTORS_KEY: &str = "trainer::n_factors";
    const VEHICLE_LR_KEY: &str = "trainer::vehicle_lr";
    const ACCOUNT_LR_KEY: &str = "trainer::account_lr";
    const REGULARIZATION_KEY: &str = "trainer::r";

    if let Ok(Some(n_factors)) = redis.get::<_, Option<usize>>(N_FACTORS_KEY).await {
        redis.del(N_FACTORS_KEY).await?;
        log::warn!("Setting factor count to {}.", n_factors);
        opts.n_factors = n_factors;
    }
    if let Ok(Some(rate)) = redis.get::<_, Option<f64>>(VEHICLE_LR_KEY).await {
        redis.del(VEHICLE_LR_KEY).await?;
        log::warn!("Setting vehicle learning rate to {}.", rate);
        opts.vehicle_learning_rate = rate;
    }
    if let Ok(Some(rate)) = redis.get::<_, Option<f64>>(ACCOUNT_LR_KEY).await {
        redis.del(ACCOUNT_LR_KEY).await?;
        log::warn!("Setting account learning rate to {}.", rate);
        opts.account_learning_rate = rate;
    }
    if let Ok(Some(regularization)) = redis.get::<_, Option<f64>>(REGULARIZATION_KEY).await {
        redis.del(REGULARIZATION_KEY).await?;
        log::warn!("Setting regularization to {}.", regularization);
        opts.regularization = regularization;
    }

    Ok(())
}

async fn get_batch(
    redis: &mut MultiplexedConnection,
    size: usize,
) -> crate::Result<Vec<TrainStep>> {
    log::debug!("Waiting for a batch of {} training steps…", size);
    let _stopwatch = Stopwatch::new("Retrieved a batch").level(Level::Debug);

    let mut steps = Vec::new();
    while steps.len() < size {
        let element: Option<(String, Bytes)> = redis.blpop(TRAINER_QUEUE_KEY, 60).await?;
        if let Some((_, step)) = element {
            steps.push(rmp_serde::from_read_ref(&step)?);
        } else {
            log::warn!("No train steps are being pushed to the queue.");
        }
    }

    debug_assert_eq!(steps.len(), size);
    Ok(steps)
}

fn borrow_vehicle_factors(
    cache: &mut HashMap<i32, Vec<f64>>,
    tank_id: i32,
    n_factors: usize,
) -> crate::Result<&mut Vec<f64>> {
    let tank_id = REMAP_TANK_ID.get(&tank_id).copied().unwrap_or(tank_id);
    let mut factors = cache.entry(tank_id).or_insert_with(Vec::new);
    initialize_factors(&mut factors, n_factors);
    Ok(factors)
}

const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";

pub async fn get_all_vehicle_factors(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<i32, Vec<f64>>> {
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

async fn set_vehicle_factors(
    redis: &mut MultiplexedConnection,
    vehicles_factors: &HashMap<i32, Vec<f64>>,
    tank_ids: &[i32],
) -> crate::Result {
    let mut pipeline = pipe();
    for tank_id in tank_ids.iter() {
        let bytes = rmp_serde::to_vec(&vehicles_factors[tank_id])?;
        pipeline.hset(VEHICLE_FACTORS_KEY, tank_id, &bytes);
        if let Some(tank_copy_id) = REMAP_TANK_ID.get(tank_id) {
            pipeline.hset(VEHICLE_FACTORS_KEY, *tank_copy_id, bytes);
        }
    }
    pipeline.query_async(redis).await?;
    Ok(())
}
