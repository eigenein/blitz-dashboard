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
use crate::math::statistics::mean;
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

    let baseline_error = get_baseline_error(&battles);
    tracing::info!(baseline_error = baseline_error);

    let data_state = DataState {
        redis,
        battles,
        pointer,
        baseline_error,
    };

    if opts.n_grid_search_epochs.is_none() {
        run_epochs(1.., opts, data_state).await?;
    } else {
        run_grid_search(opts, data_state).await?;
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

#[derive(Clone)]
struct DataState {
    redis: MultiplexedConnection,
    battles: Vec<(DateTime, Battle)>,
    pointer: String,
    baseline_error: f64,
}

struct TrainingState {
    vehicle_cache: HashMap<i32, Vector>,
    account_cache: LruCache<i32, Vector>,
    modified_account_ids: HashSet<i32>,
}

#[tracing::instrument(
    skip_all,
    fields(
        n_factors = opts.n_factors,
        regularization = opts.regularization,
        auto_regularization = !opts.no_auto_regularization,
        commit_period = format_duration(opts.commit_period).as_str(),
    ),
)]
async fn run_epochs(
    epochs: impl Iterator<Item = usize>,
    mut opts: TrainerOpts,
    mut data_state: DataState,
) -> crate::Result<f64> {
    let mut test_error = 0.0;
    let mut old_errors = None;
    let mut last_commit_instant = Instant::now();

    let mut training_state = TrainingState {
        vehicle_cache: HashMap::new(),
        account_cache: LruCache::unbounded(),
        modified_account_ids: HashSet::new(),
    };

    for i in epochs {
        let turbo_learning_rate = old_errors
            .map(|(_, test_error)| test_error > data_state.baseline_error)
            .unwrap_or(true);
        let (train_error, new_test_error) = run_epoch(
            i,
            &opts,
            turbo_learning_rate,
            &mut data_state,
            &mut training_state,
        )
        .await?;
        test_error = new_test_error;
        if let Some((old_train_error, old_test_error)) = old_errors {
            if !opts.no_auto_regularization && test_error > old_test_error {
                if train_error <= old_train_error {
                    opts.regularization += 0.001
                } else {
                    opts.regularization = (opts.regularization - 0.001).max(0.001)
                }
            }
        }
        old_errors = Some((train_error, test_error));

        if opts.n_grid_search_epochs.is_none()
            && last_commit_instant.elapsed() >= opts.commit_period
        {
            commit_factors(&opts, &mut data_state, &mut training_state).await?;
            last_commit_instant = Instant::now();
        }
    }

    Ok(test_error)
}

#[tracing::instrument(skip_all)]
async fn commit_factors(
    opts: &TrainerOpts,
    data_state: &mut DataState,
    training_state: &mut TrainingState,
) -> crate::Result {
    let start_instant = Instant::now();
    set_all_accounts_factors(
        &mut data_state.redis,
        &mut training_state.modified_account_ids,
        &training_state.account_cache,
        opts.account_ttl_secs,
    )
    .await?;
    training_state.account_cache.resize(opts.account_cache_size);
    set_all_vehicles_factors(&mut data_state.redis, &training_state.vehicle_cache).await?;
    tracing::info!(
        elapsed = format_elapsed(&start_instant).as_str(),
        "factors committed",
    );
    Ok(())
}

#[tracing::instrument(
    skip_all,
    fields(
        n_iterations = opts.grid_search_iterations,
        n_epochs = opts.n_grid_search_epochs.unwrap(),
    ),
)]
async fn run_grid_search(mut opts: TrainerOpts, mut data_state: DataState) -> crate::Result {
    tracing::info!("running the initial evaluation");
    let mut best_n_factors = opts.n_factors;
    let mut best_error = run_grid_search_on_parameters(&opts, &mut data_state).await?;

    tracing::info!("starting the search");
    for n_factors in &opts.grid_search_factors {
        opts.n_factors = *n_factors;
        let error = run_grid_search_on_parameters(&opts, &mut data_state).await?;
        if error < best_error {
            tracing::info!(
                error = error,
                was = best_error,
                by = best_error - error,
                "IMPROVED",
            );
            best_error = error;
            best_n_factors = *n_factors;
        } else {
            tracing::info!("no improvement");
        };
        tracing::info!(
            n_factors = best_n_factors,
            error = best_error,
            over_baseline = best_error - data_state.baseline_error,
            "BEST SO FAR",
        );
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn run_grid_search_on_parameters(
    opts: &TrainerOpts,
    data_state: &mut DataState,
) -> crate::Result<f64> {
    let start_instant = Instant::now();
    let tasks = (1..=opts.grid_search_iterations).map(|_| {
        let opts = opts.clone();
        let data_state = data_state.clone();
        tokio::spawn(async move {
            run_epochs(1..=opts.n_grid_search_epochs.unwrap(), opts, data_state).await
        })
    });
    let errors = futures::future::try_join_all(tasks)
        .await?
        .into_iter()
        .collect::<crate::Result<Vec<f64>>>()?;
    let error = mean(&errors);
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
fn get_baseline_error(battles: &[(DateTime, Battle)]) -> f64 {
    let mut error = error::Error::default();
    for (_, battle) in battles {
        error.push(0.5, battle.is_win);
    }
    error.average()
}

#[tracing::instrument(skip_all)]
async fn run_epoch(
    nr_epoch: usize,
    opts: &TrainerOpts,
    turbo_learning_rate: bool,
    data_state: &mut DataState,
    training_state: &mut TrainingState,
) -> crate::Result<(f64, f64)> {
    let start_instant = Instant::now();

    let learning_rate = if turbo_learning_rate {
        opts.turbo_learning_rate
    } else {
        opts.learning_rate
    };

    let mut train_error = error::Error::default();
    let mut test_error = error::Error::default();

    let mut n_new_accounts = 0;
    let mut n_initialized_accounts = 0;

    fastrand::shuffle(&mut data_state.battles);
    training_state.modified_account_ids.clear();

    let regularization_multiplier = learning_rate * opts.regularization;

    for (_, battle) in data_state.battles.iter() {
        let account_factors = match training_state.account_cache.get_mut(&battle.account_id) {
            Some(factors) => factors,
            None => {
                let factors = if opts.n_grid_search_epochs.is_none() {
                    get_account_factors(&mut data_state.redis, battle.account_id).await?
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
                training_state.account_cache.put(battle.account_id, factors);
                training_state
                    .account_cache
                    .get_mut(&battle.account_id)
                    .unwrap()
            }
        };

        let tank_id = remap_tank_id(battle.tank_id);
        let vehicle_factors = match training_state.vehicle_cache.entry(tank_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let factors = if opts.n_grid_search_epochs.is_none() {
                    get_vehicle_factors(&mut data_state.redis, tank_id).await?
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

            training_state
                .modified_account_ids
                .insert(battle.account_id);
            train_error.push(prediction, battle.is_win);
        } else {
            test_error.push(prediction, battle.is_win);
        }
    }

    let train_error = train_error.average();
    let test_error = test_error.average();
    let max_factor = training_state
        .vehicle_cache
        .iter()
        .flat_map(|(_, factors)| factors.iter().map(|factor| factor.abs()))
        .fold(0.0, f64::max);

    if opts.n_grid_search_epochs.is_none() {
        if let Some((_, new_pointer)) = refresh_battles(
            &mut data_state.redis,
            &data_state.pointer,
            &mut data_state.battles,
            opts.time_span,
        )
        .await?
        {
            data_state.pointer = new_pointer;
        }
    }

    if !opts.silence_epochs {
        log::info!(
            "#{} | err: {:>8.6} | test: {:>8.6} {:>+5.2}% | R: {:>5.3} | BPS: {:>3.0}k | B: {:>4.0}k | A: {:>3.0}k | I: {:>2} | N: {:>2} | MF: {:>7.4}",
            nr_epoch,
            train_error,
            test_error,
            (test_error / train_error - 1.0) * 100.0,
            opts.regularization,
            data_state.battles.len() as f64 / 1000.0 / start_instant.elapsed().as_secs_f64(),
            data_state.battles.len() as f64 / 1000.0,
            training_state.modified_account_ids.len() as f64 / 1000.0,
            n_initialized_accounts,
            n_new_accounts,
            max_factor,
        );
    }
    Ok((train_error, test_error))
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
