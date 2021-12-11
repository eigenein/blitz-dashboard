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
use crate::opts::TrainerOpts;
use crate::tankopedia::remap_tank_id;
use crate::trainer::dataset::Dataset;
use crate::trainer::math::make_gradient_descent_step;
use crate::trainer::math::predict_probability;
use crate::trainer::model::Model;
use crate::trainer::sample_point::SamplePoint;

mod dataset;
mod loss;
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
    let mut last_train_loss = f64::INFINITY;
    let mut last_test_loss = f64::INFINITY;
    let mut model = Model::new(redis, opts.model);

    let range = opts.n_grid_search_epochs.unwrap_or(usize::MAX); // FIXME
    for i in 1..=range {
        let start_instant = Instant::now();
        let (train_loss, test_loss) = run_epoch(&mut dataset, &mut model).await?;
        if opts.auto_r {
            adjust_regularization(
                last_train_loss,
                train_loss,
                last_test_loss,
                test_loss,
                &mut model.opts.regularization,
            );
        }
        last_train_loss = train_loss;
        last_test_loss = test_loss;

        if i % opts.log_epochs == 0 {
            log::info!(
                "#{} | train: {:>8.6} | test: {:>8.6} | R: {:>5.3} | SPPS: {:>3.0}k | SP: {:>4.0}k | A: {:>3.0}k | I: {:>2} | N: {:>2}",
                i,
                train_loss,
                test_loss,
                model.opts.regularization,
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

    Ok(last_test_loss)
}

fn adjust_regularization(
    last_train_loss: f64,
    train_loss: f64,
    last_test_loss: f64,
    test_loss: f64,
    regularization: &mut f64,
) {
    if test_loss > last_test_loss {
        if train_loss < last_train_loss {
            *regularization += 0.001;
        } else {
            *regularization = (*regularization - 0.001).max(0.0);
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
    let mut best_loss = f64::INFINITY;

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
        let loss = run_grid_search_on_parameters(&opts, &mut dataset, &should_stop).await?;
        if loss < best_loss {
            tracing::info!(
                loss = loss,
                was = best_loss,
                by = best_loss - loss,
                "↓ improved",
            );
            best_loss = loss;
            best_opts = Some(opts.model);
        } else {
            tracing::info!(worse_by = loss - best_loss, "⨯ no improvement");
        };
        tracing::info!(
            n_factors = best_opts.unwrap().n_factors,
            regularization = best_opts.unwrap().regularization,
            loss = best_loss,
            over_baseline = best_loss - dataset.baseline_loss,
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
    let losses = futures::future::try_join_all(tasks)
        .await?
        .into_iter()
        .collect::<crate::Result<Vec<f64>>>()?;
    let loss = losses
        .iter()
        .copied()
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);
    tracing::info!(
        n_factors = opts.model.n_factors,
        regularization = opts.model.regularization,
        min_loss = loss,
        elapsed = format_elapsed(&start_instant).as_str(),
        "✔ tested",
    );
    Ok(loss)
}

/// Run one SGD epoch on the entire dataset.
#[tracing::instrument(skip_all)]
async fn run_epoch(dataset: &mut Dataset, model: &mut Model) -> crate::Result<(f64, f64)> {
    let mut train_loss = loss::BCELoss::default();
    let mut test_loss = loss::BCELoss::default();

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
            train_loss.push_sample(prediction, label);
            for _ in 0..point.n_battles {
                prediction = logistic(make_gradient_descent_step(
                    factors.account,
                    factors.vehicle,
                    learning_rate * (label - prediction),
                    regularization_multiplier,
                ));
            }
            model.touch_account(point.account_id);
        } else {
            test_loss.push_sample(prediction, label);
        }
    }

    let train_loss = train_loss.average();
    let test_loss = test_loss.average();

    dataset.refresh().await?;

    if train_loss.is_finite() && test_loss.is_finite() {
        Ok((train_loss, test_loss))
    } else {
        Err(anyhow!(
            "the learning rate is too big, train loss = {}, test loss = {}",
            train_loss,
            test_loss,
        ))
    }
}
