//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::result::Result as StdResult;
use std::time::Instant;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use redis::aio::MultiplexedConnection;
use redis::streams::{StreamMaxlen, StreamRangeReply, StreamReadReply};
use redis::{pipe, AsyncCommands, Pipeline, Value};
use serde::{Deserialize, Serialize};

use math::{initialize_factors, predict_win_rate};

use crate::opts::TrainerOpts;
use crate::trainer::vector::Vector;

pub mod math;
pub mod vector;

pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let mut redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let (mut pointer, mut steps) = fetch_training_steps(&mut redis, opts.queue_size).await?;
    log::info!("Fetched {} steps, last ID: {}.", steps.len(), pointer);

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
            let index = fastrand::usize(0..steps.len());
            let TrainStep {
                account_id,
                tank_id,
                is_win,
            } = steps[index];

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
            account_factors.sgd_assign(
                vehicle_factors,
                residual_error,
                opts.account_learning_rate,
                opts.regularization,
            );
            vehicle_factors.sgd_assign(
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
        let moving_error = update_error_ewma(&mut redis, error, opts.ewma_factor).await?;
        let (n_pushed_steps, new_pointer) =
            refresh_training_steps(&mut redis, pointer, &mut steps, opts.queue_size).await?;
        pointer = new_pointer;
        log::info!(
            "AE: {:>+7.3} pp | EWMA: {:>+7.3} pp | {:>6.0} SPS | PS: {:>5} | IA: {:>5} | NA: {:>5}",
            error,
            moving_error,
            opts.batch_size as f64 / start_instant.elapsed().as_secs_f64(),
            n_pushed_steps,
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
    stream_size: usize,
) -> crate::Result {
    let serialized_steps: StdResult<Vec<Vec<u8>>, rmp_serde::encode::Error> =
        steps.iter().map(rmp_serde::to_vec).collect();
    let serialized_steps = serialized_steps.context("failed to serialize the steps")?;
    let maxlen = StreamMaxlen::Approx(stream_size);
    let mut pipeline = pipe();
    for step in serialized_steps {
        pipeline
            .xadd_maxlen(TRAINER_STREAM_KEY, maxlen, "*", &[("b", step)])
            .ignore();
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to add the steps to the stream")?;
    Ok(())
}

const TRAINER_STREAM_KEY: &str = "streams::steps";

/// Fetches initial training steps.
async fn fetch_training_steps(
    redis: &mut MultiplexedConnection,
    queue_size: usize,
) -> crate::Result<(String, VecDeque<TrainStep>)> {
    let mut queue = VecDeque::with_capacity(queue_size);
    let reply: StreamRangeReply = redis
        .xrevrange_count(TRAINER_STREAM_KEY, "+", "-", queue_size)
        .await?;
    let last_id = reply
        .ids
        .first()
        .map(|entry| entry.id.clone())
        .unwrap_or_else(|| "0".to_string());
    for entry in reply.ids {
        debug_assert!(entry.id <= last_id, "{} > {}", entry.id, last_id);
        // `XREVRANGE` returns the entries in the reverse order (newest first).
        // I want to have the oldest entry in the front of the queue.
        queue.push_front(map_entry_to_step(entry.map)?);
    }
    assert!(queue.len() <= queue_size);
    Ok((last_id, queue))
}

/// Fetches the recent training steps and throws away the oldest ones.
async fn refresh_training_steps(
    redis: &mut MultiplexedConnection,
    last_id: String,
    queue: &mut VecDeque<TrainStep>,
    queue_size: usize,
) -> crate::Result<(usize, String)> {
    let mut reply: StreamReadReply = redis.xread(&[TRAINER_STREAM_KEY], &[&last_id]).await?;
    let entries = match reply.keys.pop() {
        Some(key) => key.ids,
        None => return Ok((0, last_id)),
    };
    let (n_steps, last_id) = match entries.last() {
        Some(entry) => (entries.len(), entry.id.clone()),
        None => (0, last_id),
    };
    // Pop the oldest steps to make space for the newly coming steps.
    while queue.len() + n_steps > queue_size {
        queue.pop_front();
    }
    // And now push the new steps back.
    for entry in entries {
        queue.push_back(map_entry_to_step(entry.map)?);
    }
    assert!(queue.len() <= queue_size);
    Ok((n_steps, last_id))
}

fn map_entry_to_step(map: HashMap<String, Value>) -> crate::Result<TrainStep> {
    match map.get("b") {
        Some(Value::Data(bytes)) => Ok(rmp_serde::from_read_ref(&bytes)?),
        entry => Err(anyhow!("invalid entry value: {:?}", entry)),
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

pub async fn get_account_factors(
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
