//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::result::Result as StdResult;
use std::time::Instant;

use anyhow::{anyhow, Context};
use hashbrown::{hash_map::Entry, HashMap, HashSet};
use lru::LruCache;
use redis::aio::MultiplexedConnection;
use redis::streams::StreamMaxlen;
use redis::{pipe, AsyncCommands, Pipeline};

use dataset::Dataset;
use math::{initialize_factors, predict_win_rate};
use sample_point::SamplePoint;

use crate::helpers::{format_duration, format_elapsed};
use crate::math::statistics::mean;
use crate::opts::TrainerOpts;
use crate::tankopedia::remap_tank_id;
use crate::trainer::math::sgd;
use crate::Vector;

mod dataset;
mod error;
pub mod math;
pub mod sample_point;

const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";

#[tracing::instrument(
    skip_all,
    fields(
        account_ttl_secs = opts.account_ttl_secs,
        time_span = opts.time_span.to_string().as_str(),
    ),
)]
pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let dataset = Dataset::load(redis, opts.time_span).await?;

    if opts.n_grid_search_epochs.is_none() {
        run_epochs(1.., opts, dataset).await?;
    } else {
        run_grid_search(opts, dataset).await?;
    }
    Ok(())
}

pub async fn push_sample_points(
    redis: &mut MultiplexedConnection,
    points: &[SamplePoint],
    stream_size: usize,
) -> crate::Result {
    let points: StdResult<Vec<Vec<u8>>, rmp_serde::encode::Error> =
        points.iter().map(rmp_serde::to_vec).collect();
    let points = points.context("failed to serialize the battles")?;
    let maxlen = StreamMaxlen::Approx(stream_size);
    let mut pipeline = pipe();
    for point in points {
        pipeline
            .xadd_maxlen(dataset::TRAIN_STREAM_KEY, maxlen, "*", &[("b", point)])
            .ignore();
    }
    pipeline
        .query_async(redis)
        .await
        .context("failed to add the sample points to the stream")?;
    Ok(())
}

struct Model {
    vehicle_cache: HashMap<i32, Vector>,
    account_cache: LruCache<i32, Vector>,
    modified_account_ids: HashSet<i32>,
}

#[tracing::instrument(
    skip_all,
    fields(
        n_factors = opts.n_factors,
        regularization = opts.regularization,
        regularization_step = opts.regularization_step,
        factor_std = opts.factor_std,
        commit_period = format_duration(opts.commit_period).as_str(),
    ),
)]
async fn run_epochs(
    epochs: impl Iterator<Item = usize>,
    mut opts: TrainerOpts,
    mut dataset: Dataset,
) -> crate::Result<f64> {
    let mut test_error = 0.0;
    let mut old_errors = None;
    let mut last_commit_instant = Instant::now();

    let mut model = Model {
        vehicle_cache: HashMap::new(),
        account_cache: LruCache::unbounded(),
        modified_account_ids: HashSet::new(),
    };

    for i in epochs {
        let turbo_learning_rate = old_errors
            .map(|(_, test_error)| test_error > dataset.baseline_error)
            .unwrap_or(true);
        let (train_error, new_test_error) =
            run_epoch(i, &opts, turbo_learning_rate, &mut dataset, &mut model).await?;
        test_error = new_test_error;
        if let Some((old_train_error, old_test_error)) = old_errors {
            if test_error > old_test_error {
                if train_error <= old_train_error {
                    opts.regularization += opts.regularization_step;
                } else {
                    opts.regularization = (opts.regularization - opts.regularization_step)
                        .max(opts.regularization_step);
                }
            }
        }
        old_errors = Some((train_error, test_error));

        if opts.n_grid_search_epochs.is_none()
            && last_commit_instant.elapsed() >= opts.commit_period
        {
            commit_factors(&opts, &mut dataset, &mut model).await?;
            last_commit_instant = Instant::now();
        }
    }

    tracing::info!(final_regularization = opts.regularization);
    Ok(test_error)
}

#[tracing::instrument(skip_all)]
async fn commit_factors(
    opts: &TrainerOpts,
    dataset: &mut Dataset,
    model: &mut Model,
) -> crate::Result {
    let start_instant = Instant::now();
    set_all_accounts_factors(
        &mut dataset.redis,
        &mut model.modified_account_ids,
        &model.account_cache,
        opts.account_ttl_secs,
    )
    .await?;
    model.modified_account_ids.clear();
    model.account_cache.resize(opts.account_cache_size);
    set_all_vehicles_factors(&mut dataset.redis, &model.vehicle_cache).await?;
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
async fn run_grid_search(mut opts: TrainerOpts, mut dataset: Dataset) -> crate::Result {
    tracing::info!("running the initial evaluation");
    let mut best_n_factors = opts.n_factors;
    let mut best_error = run_grid_search_on_parameters(&opts, &mut dataset).await?;

    tracing::info!("starting the search");
    for n_factors in &opts.grid_search_factors {
        opts.n_factors = *n_factors;
        let error = run_grid_search_on_parameters(&opts, &mut dataset).await?;
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
            over_baseline = best_error - dataset.baseline_error,
            "BEST SO FAR",
        );
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn run_grid_search_on_parameters(
    opts: &TrainerOpts,
    dataset: &mut Dataset,
) -> crate::Result<f64> {
    let start_instant = Instant::now();
    let tasks = (1..=opts.grid_search_iterations).map(|_| {
        let opts = opts.clone();
        let dataset = dataset.clone();
        tokio::spawn(async move {
            run_epochs(1..=opts.n_grid_search_epochs.unwrap(), opts, dataset).await
        })
    });
    let errors = futures::future::try_join_all(tasks)
        .await?
        .into_iter()
        .collect::<crate::Result<Vec<f64>>>()?;
    let error = mean(&errors);
    tracing::info!(
        n_factors = opts.n_factors,
        initial_regularization = opts.regularization,
        mean_error = error,
        elapsed = format_elapsed(&start_instant).as_str(),
        "tested the parameters"
    );
    Ok(error)
}

#[tracing::instrument(skip_all)]
async fn run_epoch(
    nr_epoch: usize,
    opts: &TrainerOpts,
    turbo_learning_rate: bool,
    dataset: &mut Dataset,
    model: &mut Model,
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

    fastrand::shuffle(&mut dataset.sample);
    let regularization_multiplier = learning_rate * opts.regularization;

    for (_, point) in dataset.sample.iter() {
        let account_factors = match model.account_cache.get_mut(&point.account_id) {
            Some(factors) => factors,
            None => {
                let factors = if opts.n_grid_search_epochs.is_none() {
                    get_account_factors(&mut dataset.redis, point.account_id).await?
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
                model.account_cache.put(point.account_id, factors);
                model.account_cache.get_mut(&point.account_id).unwrap()
            }
        };

        let tank_id = remap_tank_id(point.tank_id);
        let vehicle_factors = match model.vehicle_cache.entry(tank_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let factors = if opts.n_grid_search_epochs.is_none() {
                    get_vehicle_factors(&mut dataset.redis, tank_id).await?
                } else {
                    None
                };
                let mut factors = factors.unwrap_or_else(Vector::new);
                initialize_factors(&mut factors, opts.n_factors, opts.factor_std);
                entry.insert(factors)
            }
        };

        let prediction = predict_win_rate(vehicle_factors, account_factors);
        let label = point.n_wins as f64 / point.n_battles as f64;
        let weight = point.n_battles as f64;

        if !point.is_test {
            sgd(
                account_factors,
                vehicle_factors,
                learning_rate * (label - prediction) * weight,
                regularization_multiplier * weight,
            )?;
            model.modified_account_ids.insert(point.account_id);
            train_error.push(prediction, label, weight);
        } else {
            test_error.push(prediction, label, weight);
        }
    }

    let train_error = train_error.average();
    let test_error = test_error.average();

    if opts.n_grid_search_epochs.is_none() {
        dataset.refresh().await?;
    }

    if nr_epoch % opts.log_epochs == 0 {
        log::info!(
            "#{} | err: {:>8.6} | test: {:>8.6} {:>+5.2}% | R: {:>5.3} | SPPS: {:>3.0}k | SP: {:>4.0}k | A: {:>3.0}k | I: {:>2} | N: {:>2}",
            nr_epoch,
            train_error,
            test_error,
            (test_error / train_error - 1.0) * 100.0,
            opts.regularization,
            dataset.sample.len() as f64 / 1000.0 / start_instant.elapsed().as_secs_f64(),
            dataset.sample.len() as f64 / 1000.0,
            model.modified_account_ids.len() as f64 / 1000.0,
            n_initialized_accounts,
            n_new_accounts,
        );
    }
    if train_error.is_finite() && test_error.is_finite() {
        Ok((train_error, test_error))
    } else {
        Err(anyhow!("the learning rate is too big"))
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
