//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::anyhow;
use redis::aio::MultiplexedConnection;

use crate::helpers::format_duration;
use crate::opts::TrainerOpts;
use crate::tankopedia::remap_tank_id;
use crate::trainer::dataset::Dataset;
use crate::trainer::loss::LossPair;
use crate::trainer::math::make_gradient_descent_step;
use crate::trainer::math::predict_probability;
use crate::trainer::model::Model;
use crate::Float;

pub mod dataset;
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
    let dataset = Dataset::load(redis.clone(), opts.time_span).await?;
    run_epochs(redis, opts, dataset, Arc::new(AtomicBool::new(false))).await?;

    Ok(())
}

#[tracing::instrument(
    skip_all,
    fields(
        n_factors = opts.model.n_factors,
        learning_rate = opts.model.learning_rate,
        factor_std = opts.model.factor_std,
        commit_period = format_duration(opts.model.flush_period).as_str(),
    ),
)]
async fn run_epochs(
    redis: MultiplexedConnection,
    opts: TrainerOpts,
    mut dataset: Dataset,
    should_stop: Arc<AtomicBool>,
) -> crate::Result<Float> {
    let mut last_losses = LossPair::infinity();
    let mut model = Model::new(redis, opts.model).await?;

    for nr_epoch in 1.. {
        let start_instant = Instant::now();
        let losses = run_epoch(&mut dataset, &mut model).await?;
        if opts.auto_r {
            model.regularization = adjust_regularization(
                nr_epoch,
                &last_losses,
                &losses,
                model.regularization,
                opts.auto_r_bump_chance,
            );
        }

        last_losses = losses;

        if nr_epoch % opts.log_epochs == 0 {
            log::info!(
                "#{} | train: {:>8.6} | test: {:>8.6} | R: {:>5.3} | SPPS: {:>3.0}k | SP: {:>4.0}k | A: {:>3.0}k | I: {:>2} | N: {:>2}",
                nr_epoch,
                losses.train,
                losses.test,
                model.regularization,
                dataset.sample.len() as Float / 1000.0 / start_instant.elapsed().as_secs_f64(),
                dataset.sample.len() as Float / 1000.0,
                model.n_modified_accounts() as Float / 1000.0,
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

    Ok(last_losses.test)
}

fn adjust_regularization(
    nr_epoch: usize,
    last_losses: &LossPair,
    losses: &LossPair,
    regularization: Float,
    auto_r_bump_chance: Option<Float>,
) -> Float {
    if losses.test > last_losses.test {
        return if losses.train < last_losses.train {
            regularization + 0.001
        } else {
            (regularization - 0.001).max(0.0)
        };
    }

    if let Some(auto_r_bump_chance) = auto_r_bump_chance {
        if fastrand::f64() < auto_r_bump_chance {
            tracing::warn!("#{} random regularization bump", nr_epoch);
            return regularization + 0.001;
        }
    }

    regularization
}

/// Run one SGD epoch on the entire dataset.
#[tracing::instrument(skip_all)]
async fn run_epoch(dataset: &mut Dataset, model: &mut Model) -> crate::Result<LossPair> {
    let mut losses_builder = LossPair::builder();

    fastrand::shuffle(&mut dataset.sample);
    let regularization = model.regularization;
    let learning_rate = model.opts.learning_rate;

    for (_, point) in dataset.sample.iter() {
        let factors = model
            .get_factors_mut(point.account_id, remap_tank_id(point.tank_id))
            .await?;

        let prediction = predict_probability(factors.vehicle, factors.account);
        let label = point.n_wins as Float / point.n_battles as Float;

        if !point.is_test {
            losses_builder.train.push_sample(prediction, label);
            make_gradient_descent_step(
                factors.account,
                factors.vehicle,
                label - prediction,
                regularization,
                learning_rate,
            );
            model.touch_account(point.account_id);
        } else {
            losses_builder.test.push_sample(prediction, label);
        }
    }

    let losses = losses_builder.finalise();

    dataset.refresh().await?;

    if losses.is_finite() {
        Ok(losses)
    } else {
        Err(anyhow!(
            "the learning rate is too big, train loss = {}, test loss = {}",
            losses.train,
            losses.test,
        ))
    }
}
