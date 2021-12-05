//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::result::Result as StdResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context};
use itertools::Itertools;
use redis::aio::MultiplexedConnection;
use redis::pipe;
use redis::streams::StreamMaxlen;
use tokio::task::JoinHandle;

use crate::helpers::{format_duration, format_elapsed};
use crate::math::logistic;
use crate::math::statistics::mean;
use crate::opts::TrainerOpts;
use crate::tankopedia::remap_tank_id;
use crate::trainer::dataset::Dataset;
use crate::trainer::math::make_gradient_descent_step;
use crate::trainer::math::predict_probability;
use crate::trainer::model::Model;
use crate::trainer::sample_point::SamplePoint;

mod dataset;
mod error;
pub mod math;
pub mod model;
pub mod sample_point;

#[tracing::instrument(
    skip_all,
    fields(
        account_ttl_secs = opts.model.account_ttl_secs,
        time_span = opts.time_span.to_string().as_str(),
    ),
)]
pub async fn run(opts: TrainerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let redis = redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let dataset = Dataset::load(
        redis.clone(),
        opts.time_span,
        opts.n_grid_search_epochs.is_none(),
    )
    .await?;

    if opts.n_grid_search_epochs.is_none() {
        run_epochs(Some(redis), opts, dataset, Arc::new(AtomicBool::new(false))).await?;
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

#[tracing::instrument(
    skip_all,
    fields(
        n_factors = opts.model.n_factors,
        regularization = opts.model.regularization,
        factor_std = opts.model.factor_std,
        commit_period = format_duration(opts.model.flush_period).as_str(),
    ),
)]
async fn run_epochs(
    redis: Option<MultiplexedConnection>,
    opts: TrainerOpts,
    mut dataset: Dataset,
    should_stop: Arc<AtomicBool>,
) -> crate::Result<f64> {
    let mut last_train_error = None;
    let mut last_test_error = None;
    let mut model = Model::new(redis, opts.model);

    let range = opts.n_grid_search_epochs.unwrap_or(usize::MAX); // FIXME
    for i in 1..=range {
        let start_instant = Instant::now();
        let (train_error, test_error) = run_epoch(&mut dataset, &mut model).await?;
        if let Some(auto_r) = opts.auto_r {
            if i % auto_r == 0 {
                adjust_regularization(
                    last_train_error,
                    train_error,
                    last_test_error,
                    test_error,
                    &mut model.opts.regularization,
                );
                last_train_error = Some(train_error);
                last_test_error = Some(test_error);
            }
        } else if i == range {
            last_test_error = Some(test_error);
        }

        if i % opts.log_epochs == 0 {
            log::info!(
                "#{} | train: {:>8.6} | test: {:>8.6} | SPPS: {:>3.0}k | SP: {:>4.0}k | A: {:>3.0}k | I: {:>2} | N: {:>2}",
                i,
                train_error,
                test_error,
                dataset.sample.len() as f64 / 1000.0 / start_instant.elapsed().as_secs_f64(),
                dataset.sample.len() as f64 / 1000.0,
                model.n_modified_accounts() as f64 / 1000.0,
                model.n_initialized_accounts,
                model.n_new_accounts,
            );
            model.n_initialized_accounts = 0;
            model.n_new_accounts = 0;
        }
        if should_stop.load(Ordering::Relaxed) {
            tracing::warn!("interrupted");
            break;
        }

        model.flush().await?;
    }

    Ok(last_test_error.unwrap())
}

fn adjust_regularization(
    last_train_error: Option<f64>,
    train_error: f64,
    last_test_error: Option<f64>,
    test_error: f64,
    regularization: &mut f64,
) {
    if let Some(last_train_error) = last_train_error {
        if let Some(last_test_error) = last_test_error {
            if test_error > last_test_error {
                if train_error < last_train_error {
                    *regularization += 0.001;
                    tracing::warn!(regularization = regularization, "increased");
                } else {
                    *regularization = (*regularization - 0.001).max(0.001);
                    tracing::warn!(regularization = regularization, "decreased");
                }
            }
        }
    }
}

/// Run the grid search on all the specified parameter sets.
#[tracing::instrument(
    skip_all,
    fields(
        n_iterations = opts.grid_search_iterations,
        n_epochs = opts.n_grid_search_epochs.unwrap(),
        n_parameter_sets = opts.grid_search_factors.len() * opts.grid_search_regularizations.len(),
    ),
)]
async fn run_grid_search(mut opts: TrainerOpts, mut dataset: Dataset) -> crate::Result {
    let mut best_opts = None;
    let mut best_error = f64::INFINITY;

    let should_stop = Arc::new(AtomicBool::default());
    {
        let should_stop = should_stop.clone();
        ctrlc::set_handler(move || {
            if should_stop.swap(true, Ordering::Relaxed) {
                tracing::warn!("repeated Ctrl+C – exiting");
                std::process::exit(1);
            }
            tracing::warn!("interrupting… Ctrl+C again to exit");
        })?;
    }

    for (i, (n_factors, regularization)) in opts
        .grid_search_factors
        .iter()
        .cartesian_product(&opts.grid_search_regularizations)
        .enumerate()
    {
        tracing::info!(run = i + 1, "starting");

        opts.model.n_factors = *n_factors;
        opts.model.regularization = *regularization;

        should_stop.store(false, Ordering::Relaxed);
        let error = run_grid_search_on_parameters(&opts, &mut dataset, &should_stop).await?;
        if error < best_error {
            tracing::info!(
                error = error,
                was = best_error,
                by = best_error - error,
                "↓ improved",
            );
            best_error = error;
            best_opts = Some(opts.model);
        } else {
            tracing::info!(worse_by = error - best_error, "⨯ no improvement");
        };
        tracing::info!(
            n_factors = best_opts.unwrap().n_factors,
            regularization = best_opts.unwrap().regularization,
            error = best_error,
            over_baseline = best_error - dataset.baseline_error,
            "= best so far",
        );
    }

    Ok(())
}

/// Run the grid search with the specified parameters.
#[tracing::instrument(skip_all)]
async fn run_grid_search_on_parameters(
    opts: &TrainerOpts,
    dataset: &mut Dataset,
    should_stop: &Arc<AtomicBool>,
) -> crate::Result<f64> {
    let start_instant = Instant::now();
    let tasks: Vec<JoinHandle<crate::Result<f64>>> = (1..=opts.grid_search_iterations)
        .map(|_| {
            tokio::spawn(run_epochs(
                None,
                opts.clone(),
                dataset.clone(),
                should_stop.clone(),
            ))
        })
        .collect();
    let errors = futures::future::try_join_all(tasks)
        .await?
        .into_iter()
        .collect::<crate::Result<Vec<f64>>>()?;
    let error = mean(&errors);
    tracing::info!(
        n_factors = opts.model.n_factors,
        regularization = opts.model.regularization,
        mean_error = error,
        elapsed = format_elapsed(&start_instant).as_str(),
        "✔ tested",
    );
    Ok(error)
}

/// Run one SGD epoch on the entire dataset.
#[tracing::instrument(skip_all)]
async fn run_epoch(dataset: &mut Dataset, model: &mut Model) -> crate::Result<(f64, f64)> {
    let mut train_error = error::Error::default();
    let mut test_error = error::Error::default();

    fastrand::shuffle(&mut dataset.sample);
    let learning_rate = model.opts.learning_rate;
    let regularization_multiplier = learning_rate * model.opts.regularization;

    for (_, point) in dataset.sample.iter() {
        let factors = model
            .get_factors_mut(point.account_id, remap_tank_id(point.tank_id))
            .await?;

        let mut prediction = predict_probability(factors.vehicle, factors.account);
        let label = point.n_wins as f64 / point.n_battles as f64;

        if !point.is_test {
            for _ in 0..point.n_battles {
                train_error.push(prediction, label, 1.0);
                prediction = logistic(make_gradient_descent_step(
                    factors.account,
                    factors.vehicle,
                    learning_rate * (label - prediction),
                    regularization_multiplier,
                ));
            }
            model.touch_account(point.account_id);
        } else {
            test_error.push(prediction, label, point.n_battles as f64);
        }
    }

    let train_error = train_error.average();
    let test_error = test_error.average();

    dataset.refresh().await?;

    if train_error.is_finite() && test_error.is_finite() {
        Ok((train_error, test_error))
    } else {
        Err(anyhow!(
            "the learning rate is too big, train error = {}, test_error = {}",
            train_error,
            test_error,
        ))
    }
}
