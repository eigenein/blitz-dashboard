//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::result::Result as StdResult;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{anyhow, Context};
use chrono::{Duration, TimeZone, Utc};
use hashbrown::{hash_map::Entry, HashMap, HashSet};
use lru::LruCache;
use redis::aio::MultiplexedConnection;
use redis::streams::{StreamMaxlen, StreamReadOptions};
use redis::{pipe, AsyncCommands, Pipeline, Value};

use battle::Battle;
use math::{initialize_factors, predict_win_rate};

use crate::helpers::{format_duration, format_elapsed};
use crate::opts::TrainerOpts;
use crate::tankopedia::remap_tank_id;
use crate::trainer::math::sgd;
use crate::{DateTime, Vector};

pub mod battle;
mod error;
pub mod math;

const TRAIN_STREAM_KEY: &str = "streams::steps";
const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";
const REFRESH_BATTLES_MAX_COUNT: usize = 250000;

#[tracing::instrument(
    skip_all,
    fields(
        account_ttl_secs = opts.account_ttl_secs,
        time_span = opts.time_span.to_string().as_str(),
    ),
)]
pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let mut redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let (pointer, battles) = load_battles(&mut redis, opts.time_span).await?;
    tracing::info!(
        n_battles = battles.len(),
        pointer = pointer.as_str(),
        "loaded",
    );

    let mut state = State {
        redis,
        battles,
        pointer,
        vehicle_cache: HashMap::new(),
        account_cache: LruCache::unbounded(),
        modified_account_ids: HashSet::new(),
    };
    if opts.n_grid_search_epochs.is_none() {
        run_epochs(1.., &opts, &mut state).await?;
    } else {
        run_grid_search(opts, state).await?;
    }
    Ok(())
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

struct State {
    redis: MultiplexedConnection,
    battles: Vec<(DateTime, Battle)>,
    pointer: String,
    vehicle_cache: HashMap<i32, Vector>,
    account_cache: LruCache<i32, Vector>,
    modified_account_ids: HashSet<i32>,
}

#[tracing::instrument(
    skip_all,
    fields(
        n_factors = opts.n_factors,
        regularization = opts.regularization,
    ),
)]
async fn run_epochs(
    epochs: impl Iterator<Item = usize>,
    opts: &TrainerOpts,
    state: &mut State,
) -> crate::Result<f64> {
    let mut error = 0.0;
    for i in epochs {
        let start_instant = Instant::now();
        error = run_epoch(i, opts, state).await?;
        if i == 1 {
            tracing::info!(elapsed_per_epoch = format_elapsed(&start_instant).as_str());
        }
    }
    Ok(error)
}

#[tracing::instrument(
    skip_all,
    fields(
        n_iterations = opts.grid_search_iterations,
        n_epochs = opts.n_grid_search_epochs.unwrap(),
    ),
)]
async fn run_grid_search(opts: TrainerOpts, mut state: State) -> crate::Result {
    let baseline_error = get_baseline_error(&state);
    tracing::info!(baseline_error = baseline_error);

    tracing::info!("running the initial evaluation");
    let mut best_opts = opts.clone();
    let mut best_error = run_grid_search_on_parameters(&opts, &mut state).await?;

    tracing::info!("starting the search");
    'search_loop: loop {
        let mut trial_opts = vec![
            TrainerOpts {
                n_factors: best_opts.n_factors - 1,
                ..best_opts.clone()
            },
            TrainerOpts {
                n_factors: best_opts.n_factors + 1,
                ..best_opts.clone()
            },
            TrainerOpts {
                regularization: 0.9 * best_opts.regularization,
                ..best_opts.clone()
            },
            TrainerOpts {
                regularization: 1.1 * best_opts.regularization,
                ..best_opts.clone()
            },
        ];
        fastrand::shuffle(&mut trial_opts);

        for opts in trial_opts {
            if opts.n_factors < 1 {
                continue;
            }
            let error = run_grid_search_on_parameters(&opts, &mut state).await?;
            let is_improved = error < best_error;
            if is_improved {
                tracing::info!(
                    error = error,
                    was = best_error,
                    by = best_error - error,
                    "IMPROVED",
                );
                best_error = error;
                best_opts = opts.clone();
            } else {
                tracing::info!("no improvement");
            };
            tracing::info!(
                n_factors = best_opts.n_factors,
                regularization = best_opts.regularization,
                error = best_error,
                over_baseline = best_error - baseline_error,
                "BEST SO FAR",
            );
            if is_improved {
                continue 'search_loop;
            }
        }
        break 'search_loop;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn run_grid_search_on_parameters(
    opts: &TrainerOpts,
    state: &mut State,
) -> crate::Result<f64> {
    let start_instant = Instant::now();
    let mut errors = Vec::with_capacity(opts.grid_search_iterations);
    for i in 1..=opts.grid_search_iterations {
        tracing::info!(iteration = i, of = opts.grid_search_iterations, "starting");
        state.account_cache.clear();
        state.vehicle_cache.clear();
        let start_instant = Instant::now();
        let error = run_epochs(1..=opts.n_grid_search_epochs.unwrap(), opts, state).await?;
        tracing::info!(elapsed = format_elapsed(&start_instant).as_str());
        errors.push(error);
    }
    let error = errors.iter().sum::<f64>() / errors.len() as f64;
    tracing::info!(
        n_factors = opts.n_factors,
        regularization = opts.regularization,
        mean_error = error,
        elapsed = format_elapsed(&start_instant).as_str(),
        "tested the parameters"
    );
    Ok(error)
}

#[tracing::instrument(skip_all)]
fn get_baseline_error(state: &State) -> f64 {
    let mut error = error::Error::default();
    for (_, battle) in &state.battles {
        error.push(0.5, battle.is_win);
    }
    error.average()
}

#[tracing::instrument(skip_all)]
async fn run_epoch(nr_epoch: usize, opts: &TrainerOpts, state: &mut State) -> crate::Result<f64> {
    let start_instant = Instant::now();

    let learning_rate = match opts.boost_learning_rate {
        Some(n_epochs) if nr_epoch <= n_epochs => 10.0 * opts.learning_rate,
        _ => opts.learning_rate,
    };

    let mut train_error = error::Error::default();
    let mut test_error = error::Error::default();

    let mut n_new_accounts = 0;
    let mut n_initialized_accounts = 0;

    fastrand::shuffle(&mut state.battles);
    state.modified_account_ids.clear();

    let regularization_multiplier = learning_rate * opts.regularization;

    for (_, battle) in state.battles.iter() {
        let account_factors = match state.account_cache.get_mut(&battle.account_id) {
            Some(factors) => factors,
            None => {
                let factors = if opts.n_grid_search_epochs.is_none() {
                    get_account_factors(&mut state.redis, battle.account_id).await?
                } else {
                    None
                };
                let mut factors = factors.unwrap_or_else(|| {
                    n_new_accounts += 1;
                    Vector::new()
                });
                if initialize_factors(&mut factors, opts.n_factors, opts.factor_std) {
                    n_initialized_accounts += 1;
                }
                state.account_cache.put(battle.account_id, factors);
                state.account_cache.get_mut(&battle.account_id).unwrap()
            }
        };

        let tank_id = remap_tank_id(battle.tank_id);
        let vehicle_factors = match state.vehicle_cache.entry(tank_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let factors = if opts.n_grid_search_epochs.is_none() {
                    get_vehicle_factors(&mut state.redis, tank_id).await?
                } else {
                    None
                };
                let mut factors = factors.unwrap_or_else(Vector::new);
                initialize_factors(&mut factors, opts.n_factors, opts.factor_std);
                entry.insert(factors)
            }
        };

        let prediction = predict_win_rate(vehicle_factors, account_factors);

        if !battle.is_test {
            let target = if battle.is_win { 1.0 } else { 0.0 };
            let residual_multiplier = learning_rate * (target - prediction);
            sgd(
                account_factors,
                vehicle_factors,
                residual_multiplier,
                regularization_multiplier,
            );

            state.modified_account_ids.insert(battle.account_id);
            train_error.push(prediction, battle.is_win);
        } else {
            test_error.push(prediction, battle.is_win);
        }
    }

    let n_accounts = state.modified_account_ids.len();
    if opts.n_grid_search_epochs.is_none() {
        set_all_accounts_factors(
            &mut state.redis,
            &mut state.modified_account_ids,
            &state.account_cache,
            opts.account_ttl_secs,
        )
        .await?;
        state.account_cache.resize(opts.account_cache_size);
        set_all_vehicles_factors(&mut state.redis, &state.vehicle_cache).await?;
    }

    let train_error = train_error.average();
    let test_error = test_error.average();
    let max_factor = state
        .vehicle_cache
        .iter()
        .flat_map(|(_, factors)| factors.iter().map(|factor| factor.abs()))
        .fold(0.0, f64::max);

    if opts.n_grid_search_epochs.is_none() {
        if let Some((_, new_pointer)) = refresh_battles(
            &mut state.redis,
            &state.pointer,
            &mut state.battles,
            opts.time_span,
        )
        .await?
        {
            state.pointer = new_pointer;
        }
    }

    if !opts.silence_epochs {
        log::info!(
        "#{} | err: {:>8.6} | test: {:>8.6} {:>+5.2}% | BPS: {:>3.0}k | B: {:>4.0}k | A: {:>3.0}k | I: {:>2} | N: {:>2} | MF: {:>7.4}",
        nr_epoch,
        train_error,
        test_error,
        (test_error / train_error - 1.0) * 100.0,
        state.battles.len() as f64 / 1000.0 / start_instant.elapsed().as_secs_f64(),
        state.battles.len() as f64 / 1000.0,
        n_accounts as f64 / 1000.0,
        n_initialized_accounts,
        n_new_accounts,
        max_factor,
    );
    }
    Ok(test_error)
}

#[tracing::instrument(skip_all, fields(time_span = format_duration(time_span.to_std()?).as_str()))]
async fn load_battles(
    redis: &mut MultiplexedConnection,
    time_span: Duration,
) -> crate::Result<(String, Vec<(DateTime, Battle)>)> {
    let mut battles = Vec::new();
    let mut pointer = (Utc::now() - time_span).timestamp_millis().to_string();

    while match refresh_battles(redis, &pointer, &mut battles, time_span).await? {
        Some((n_battles, new_pointer)) => {
            tracing::info!(n_battles = battles.len(), pointer = new_pointer.as_str());
            pointer = new_pointer;
            n_battles >= REFRESH_BATTLES_MAX_COUNT
        }
        None => false,
    } {}

    match battles.is_empty() {
        false => Ok((pointer, battles)),
        true => Err(anyhow!("training set is empty, try a longer time span")),
    }
}

#[tracing::instrument(level = "debug", skip(redis, queue, time_span))]
async fn refresh_battles(
    redis: &mut MultiplexedConnection,
    last_id: &str,
    queue: &mut Vec<(DateTime, Battle)>,
    time_span: Duration,
) -> crate::Result<Option<(usize, String)>> {
    // Remove the expired battles.
    let expire_time = Utc::now() - time_span;
    queue.retain(|(timestamp, _)| timestamp > &expire_time);

    // Fetch new battles.
    let options = StreamReadOptions::default().count(REFRESH_BATTLES_MAX_COUNT);
    let reply: Value = redis
        .xread_options(&[TRAIN_STREAM_KEY], &[&last_id], &options)
        .await?;
    let entries = parse_multiple_streams(reply)?;
    let result = entries.last().map(|(id, _)| (entries.len(), id.clone()));
    for (id, battle) in entries {
        queue.push((parse_entry_id(&id)?, battle));
    }
    tracing::debug!(n_battles = result.as_ref().map(|result| result.0));
    Ok(result)
}

fn parse_entry_id(id: &str) -> crate::Result<DateTime> {
    let millis = id
        .split_once("-")
        .ok_or_else(|| anyhow!("unexpected stream entry ID"))?
        .0;
    Ok(Utc.timestamp_millis(i64::from_str(millis)?))
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
    let bytes: Option<Vec<u8>> = redis.hget(VEHICLE_FACTORS_KEY, tank_id).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

pub async fn get_all_vehicle_factors(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<i32, Vector>> {
    let hash_map: std::collections::HashMap<i32, Vec<u8>> =
        redis.hgetall(VEHICLE_FACTORS_KEY).await?;
    hash_map
        .into_iter()
        .map(|(tank_id, value)| Ok((tank_id, rmp_serde::from_read_ref(&value)?)))
        .collect()
}

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

#[inline]
fn set_account_factors(
    pipeline: &mut Pipeline,
    account_id: i32,
    factors: &[f64],
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
    account_ids: &mut HashSet<i32>,
    cache: &LruCache<i32, Vector>,
    ttl_secs: usize,
) -> crate::Result {
    let mut pipeline = pipe();
    for account_id in account_ids.drain() {
        set_account_factors(
            &mut pipeline,
            account_id,
            cache.peek(&account_id).unwrap(),
            ttl_secs,
        )?;
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to update the accounts factors")
}
