//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::result::Result as StdResult;
use std::time::Instant;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use cached::{Cached, TimedSizedCache};
use redis::aio::MultiplexedConnection;
use redis::streams::StreamMaxlen;
use redis::{pipe, AsyncCommands, Pipeline, Value};

use math::{initialize_factors, predict_win_rate};

use crate::opts::TrainerOpts;
use battle::Battle;
use vector::Vector;

pub mod battle;
mod error;
pub mod math;
pub mod vector;

const TRAINER_TRAIN_ERROR_KEY: &str = "trainer::errors::train";
const TRAINER_TEST_ERROR_KEY: &str = "trainer::errors::test";
const TRAIN_STREAM_KEY: &str = "streams::steps";
const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";

pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let mut redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    log::info!("Loading battles…");
    let (mut pointer, mut battles) = load_battles(&mut redis, opts.train_size).await?;
    log::info!("Loaded {} battles, last ID: {}.", battles.len(), pointer);

    let account_ttl_secs: usize = opts.account_ttl.as_secs().try_into()?;
    let mut vehicle_cache = HashMap::new();
    let mut account_cache =
        TimedSizedCache::with_size_and_lifespan_and_refresh(opts.account_cache_size, 3600, true);
    let mut modified_account_ids = HashSet::new();

    log::info!("Running…");
    loop {
        let start_instant = Instant::now();

        let mut train_error = error::Error::default();
        let mut test_error = error::Error::default();

        let mut n_new_accounts = 0;
        let mut n_initialized_accounts = 0;

        fastrand::shuffle(battles.make_contiguous());
        modified_account_ids.clear();

        for battle in battles.iter() {
            let account_factors = match account_cache.cache_get_mut(&battle.account_id) {
                Some(factors) => factors,
                None => {
                    let mut factors = get_account_factors(&mut redis, battle.account_id)
                        .await?
                        .unwrap_or_else(|| {
                            n_new_accounts += 1;
                            Vector::new()
                        });
                    if initialize_factors(&mut factors, opts.n_factors, opts.factor_std) {
                        n_initialized_accounts += 1;
                    }
                    // Surprisingly, this is faster than `try_get_or_set_with`.
                    account_cache.cache_set(battle.account_id, factors);
                    account_cache.cache_get_mut(&battle.account_id).unwrap()
                }
            };

            let vehicle_factors = match vehicle_cache.entry(battle.tank_id) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let mut factors = get_vehicle_factors(&mut redis, battle.tank_id)
                        .await?
                        .unwrap_or_else(Vector::new);
                    initialize_factors(&mut factors, opts.n_factors, opts.factor_std);
                    entry.insert(factors)
                }
            };

            let prediction = predict_win_rate(vehicle_factors, account_factors);
            let target = if battle.is_win { 1.0 } else { 0.0 };
            let residual_error = target - prediction;

            if !battle.is_test {
                modified_account_ids.insert(battle.account_id);
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

                train_error.push(-residual_error);

                if let Some(duplicate_id) = REMAP_TANK_ID.get(&battle.tank_id) {
                    let vehicle_factors = vehicle_factors.clone();
                    vehicle_cache.insert(*duplicate_id, vehicle_factors);
                }
            } else {
                test_error.push(-residual_error);
            }
        }

        set_all_accounts_factors(
            &mut redis,
            &modified_account_ids,
            &mut account_cache,
            account_ttl_secs,
        )
        .await?;
        set_all_vehicles_factors(&mut redis, &vehicle_cache).await?;

        let train_error = train_error.average();
        let test_error = test_error.average();
        set_errors(&mut redis, train_error, test_error).await?;

        let max_factor = vehicle_cache
            .iter()
            .flat_map(|(_, factors)| factors.0.iter().map(|factor| factor.abs()))
            .fold(0.0, f64::max);
        let (n_new_battles, new_pointer) =
            refresh_battles(&mut redis, pointer, &mut battles, opts.train_size).await?;
        pointer = new_pointer;

        log::info!(
            "Err: {:>+9.6} pp | test: {:>+6.3} pp | BPS: {:>3.0}k {:>+5} | A: {:>3.0}k | I: {:>2} | N: {:>2} | MF: {:>7.4}",
            train_error * 100.0,
            test_error * 100.0,
            battles.len() as f64 / 1000.0 / start_instant.elapsed().as_secs_f64(),
            n_new_battles,
            modified_account_ids.len() as f64 / 1000.0,
            n_initialized_accounts,
            n_new_accounts,
            max_factor,
        );
    }
}

pub async fn get_test_error(redis: &mut MultiplexedConnection) -> crate::Result<f64> {
    Ok(redis
        .get::<_, Option<f64>>(TRAINER_TEST_ERROR_KEY)
        .await?
        .unwrap_or_default())
}

async fn set_errors(
    redis: &mut MultiplexedConnection,
    train_error: f64,
    test_error: f64,
) -> crate::Result {
    pipe()
        .set(TRAINER_TRAIN_ERROR_KEY, train_error)
        .ignore()
        .set(TRAINER_TEST_ERROR_KEY, test_error)
        .ignore()
        .query_async(redis)
        .await
        .context("failed to set the errors")
}

pub async fn push_battles(
    redis: &mut MultiplexedConnection,
    battles: &[Battle],
    stream_size: usize,
) -> crate::Result {
    let battles: StdResult<Vec<Vec<u8>>, rmp_serde::encode::Error> =
        battles.iter().map(rmp_serde::to_vec).collect();
    let battles = battles.context("failed to serialize the battles")?;
    let maxlen = StreamMaxlen::Approx(stream_size);
    let mut pipeline = pipe();
    for battle in battles {
        pipeline
            .xadd_maxlen(TRAIN_STREAM_KEY, maxlen, "*", &[("b", battle)])
            .ignore();
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to add the battles to the stream")?;
    Ok(())
}

async fn load_battles(
    redis: &mut MultiplexedConnection,
    count: usize,
) -> crate::Result<(String, VecDeque<Battle>)> {
    let mut queue = VecDeque::with_capacity(count);
    let reply: Value = redis
        .xrevrange_count(TRAIN_STREAM_KEY, "+", "-", count)
        .await?;
    log::info!("Almost done…");
    let entries = parse_stream(reply)?;
    let last_id = entries
        .first()
        .map_or_else(|| "0".to_string(), |entry| entry.0.clone());
    for (_, step) in entries {
        // `XREVRANGE` returns the entries in the reverse order (newest first).
        // I want to have the oldest entry in the front of the queue.
        queue.push_front(step);
    }
    assert!(queue.len() <= count);
    Ok((last_id, queue))
}

/// Fetches the recent battles and throws away the oldest ones.
async fn refresh_battles(
    redis: &mut MultiplexedConnection,
    last_id: String,
    queue: &mut VecDeque<Battle>,
    queue_size: usize,
) -> crate::Result<(usize, String)> {
    let reply: Value = redis.xread(&[TRAIN_STREAM_KEY], &[&last_id]).await?;
    let entries = parse_multiple_streams(reply)?;
    let (n_battles, last_id) = match entries.last() {
        Some((id, _)) => (entries.len(), id.clone()),
        None => (0, last_id),
    };
    // Pop the oldest battles to make space for the newly coming steps.
    while queue.len() + n_battles > queue_size {
        queue.pop_front();
    }
    // And now push the new battles back.
    for (_, step) in entries {
        queue.push_back(step);
    }
    assert!(queue.len() <= queue_size);
    Ok((n_battles, last_id))
}

fn parse_multiple_streams(reply: Value) -> crate::Result<Vec<(String, Battle)>> {
    match reply {
        Value::Nil => Ok(Vec::new()),
        Value::Bulk(mut streams) => match streams.pop() {
            Some(Value::Bulk(mut stream)) => match stream.pop() {
                Some(value) => parse_stream(value),
                other => Err(anyhow!("expected entries, got: {:?}", other)),
            },
            other => Err(anyhow!("expected (name, entries), got: {:?}", other)),
        },
        other => Err(anyhow!("expected a bulk of streams, got: {:?}", other)),
    }
}

fn parse_stream(reply: Value) -> crate::Result<Vec<(String, Battle)>> {
    match reply {
        Value::Nil => Ok(Vec::new()),
        Value::Bulk(entries) => entries.into_iter().map(parse_stream_entry).collect(),
        other => Err(anyhow!("expected a bulk of entries, got: {:?}", other)),
    }
}

fn parse_stream_entry(reply: Value) -> crate::Result<(String, Battle)> {
    match reply {
        Value::Bulk(mut entry) => {
            let fields = entry.pop();
            let id = entry.pop();
            match (id, fields) {
                (Some(Value::Data(id)), Some(Value::Bulk(mut fields))) => {
                    let value = fields.pop();
                    match value {
                        Some(Value::Data(data)) => {
                            Ok((String::from_utf8(id)?, rmp_serde::from_read_ref(&data)?))
                        }
                        other => Err(anyhow!("expected a binary data, got: {:?}", other)),
                    }
                }
                other => Err(anyhow!("expected (ID, fields), got: {:?}", other)),
            }
        }
        other => Err(anyhow!("expected (ID, fields), got: {:?}", other)),
    }
}

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
    account_ids: &HashSet<i32>,
    cache: &mut TimedSizedCache<i32, Vector>,
    ttl_secs: usize,
) -> crate::Result {
    let mut pipeline = pipe();
    for account_id in account_ids.iter().copied() {
        set_account_factors(
            &mut pipeline,
            account_id,
            cache
                .cache_get(&account_id)
                .ok_or_else(|| anyhow!("#{} is missing in the cache", account_id))?,
            ttl_secs,
        )?;
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to update the accounts factors")
}
