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
use lru::LruCache;
use redis::aio::MultiplexedConnection;
use redis::streams::StreamMaxlen;
use redis::{pipe, AsyncCommands, Pipeline, Value};
use serde::{Deserialize, Serialize};

use math::{initialize_factors, predict_win_rate};

use crate::opts::TrainerOpts;
use crate::trainer::vector::Vector;

mod error;
pub mod math;
pub mod vector;

pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let mut redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    log::info!("Loading battles…");
    let (mut pointer, mut battles) = load_battles(&mut redis, opts.train_size).await?;
    log::info!("Loaded {} battles, last ID: {}.", battles.len(), pointer);

    let account_ttl_secs: usize = opts.account_ttl.as_secs().try_into()?;
    let mut vehicle_factors_cache = HashMap::new();
    let mut account_factors_cache = LruCache::new(opts.account_cache_size.max(opts.batch_size));

    log::info!("Running…");
    loop {
        let start_instant = Instant::now();

        let mut train_error = error::Error::default();
        let mut test_error = error::Error::default();

        let mut modified_account_ids = HashSet::with_capacity(opts.batch_size);
        let mut n_new_accounts = 0;
        let mut n_initialized_accounts = 0;

        for _ in 0..opts.batch_size {
            let index = fastrand::usize(0..battles.len());
            let Battle {
                account_id,
                tank_id,
                is_win,
                is_test,
            } = battles[index];

            if !account_factors_cache.contains(&account_id) {
                let mut factors = get_account_factors(&mut redis, account_id)
                    .await?
                    .unwrap_or_else(|| {
                        if !is_test {
                            n_new_accounts += 1;
                        }
                        Vector::new()
                    });
                if initialize_factors(&mut factors, opts.n_factors, opts.factor_std) && !is_test {
                    n_initialized_accounts += 1;
                }
                account_factors_cache.put(account_id, factors);
            }
            let account_factors = account_factors_cache
                .get_mut(&account_id)
                .ok_or_else(|| anyhow!("#{} is missing in the cache", account_id))?;

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

            if !is_test {
                modified_account_ids.insert(account_id);
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

                if let Some(duplicate_id) = REMAP_TANK_ID.get(&tank_id) {
                    let vehicle_factors = vehicle_factors.clone();
                    vehicle_factors_cache.insert(*duplicate_id, vehicle_factors);
                }
            } else {
                test_error.push(-residual_error);
            }
        }

        let n_modified_accounts = modified_account_ids.len();
        set_all_accounts_factors(
            &mut redis,
            modified_account_ids,
            &account_factors_cache,
            account_ttl_secs,
        )
        .await?;
        set_all_vehicles_factors(&mut redis, &vehicle_factors_cache).await?;

        let (smoothed_train_error, average_train_error) = train_error
            .smooth(&mut redis, "trainer::errors::train", opts.error_smoothing)
            .await?;
        let (smoothed_test_error, average_test_error) = test_error
            .smooth(&mut redis, "trainer::errors::test", opts.error_smoothing)
            .await?;
        let max_factor = vehicle_factors_cache
            .iter()
            .flat_map(|(_, factors)| factors.0.iter().map(|factor| factor.abs()))
            .fold(0.0, f64::max);
        let (n_new_battles, new_pointer) =
            refresh_battles(&mut redis, pointer, &mut battles, opts.train_size).await?;
        pointer = new_pointer;
        log::info!(
            "Train: {:>+10.6} ({:>+.2}) pp | test: {:>+7.3} ({:>+.1}) pp | BPS: {:>6.0} | new: {:>3} | acc: {:>6} | i: {:>2} | n: {:>2} | MF: {:>6.3}",
            smoothed_train_error * 100.0,
            average_train_error * 100.0,
            smoothed_test_error * 100.0,
            average_test_error * 100.0,
            opts.batch_size as f64 / start_instant.elapsed().as_secs_f64(),
            n_new_battles,
            n_modified_accounts,
            n_initialized_accounts,
            n_new_accounts,
            max_factor,
        );
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Battle {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_win: bool,
    pub is_test: bool,
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

const TRAIN_STREAM_KEY: &str = "streams::steps";

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
    account_ids: HashSet<i32>,
    cache: &LruCache<i32, Vector>,
    ttl_secs: usize,
) -> crate::Result {
    let mut pipeline = pipe();
    for account_id in account_ids {
        set_account_factors(
            &mut pipeline,
            account_id,
            cache
                .peek(&account_id)
                .ok_or_else(|| anyhow!("#{} is missing in the cache", account_id))?,
            ttl_secs,
        )?;
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to update the accounts factors")
}
