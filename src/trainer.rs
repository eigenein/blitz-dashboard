//! Trains the account and vehicle factors on the new data.
//! Implements a stochastic gradient descent for matrix factorization.
//!
//! https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::anyhow;
use humantime::format_duration;
use redis::aio::MultiplexedConnection;
use tracing::info;

use crate::helpers::periodic::Periodic;
use crate::opts::TrainerOpts;
use crate::tankopedia::remap_tank_id;
use crate::trainer::dataset::{calculate_baseline_loss_2, calculate_vehicle_win_rates, Dataset};
use crate::trainer::loss::LossPair;
use crate::trainer::math::make_gradient_descent_step;
use crate::trainer::math::predict_probability;
use crate::trainer::model::{store_vehicle_win_rates, Model};

pub mod dataset;
mod loss;
pub mod math;
pub mod model;
pub mod sample_point;
pub mod stream_entry;

#[tracing::instrument(
    skip_all,
    fields(
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
        flush_interval = %format_duration(opts.flush_interval),
    ),
)]
async fn run_epochs(
    mut redis: MultiplexedConnection,
    opts: TrainerOpts,
    mut dataset: Dataset,
    should_stop: Arc<AtomicBool>,
) -> crate::Result<f64> {
    let mut last_losses = LossPair::infinity();
    let mut model = Model::new(redis.clone(), opts.model).await?;
    let mut periodic_flush = Periodic::new(opts.flush_interval);

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
                dataset.sample.len() as f64 / 1000.0 / start_instant.elapsed().as_secs_f64(),
                dataset.sample.len() as f64 / 1000.0,
                model.n_modified_accounts() as f64 / 1000.0,
                model.n_initialized_accounts,
                model.n_new_accounts,
            );
            model.n_initialized_accounts = 0;
            model.n_new_accounts = 0;

            let (lower_bound_loss, mean_loss, upper_bound_loss) =
                calculate_baseline_loss_2(&dataset.sample, opts.analytics_time_span);
            info!(
                lower_bound_loss = lower_bound_loss,
                mean_loss = mean_loss,
                upper_bound_loss = upper_bound_loss,
            );
        }

        if should_stop.load(Ordering::Relaxed) {
            tracing::warn!("interrupted");
            break;
        }

        if periodic_flush.should_trigger() {
            model.flush().await?;
            let vehicle_win_rates =
                calculate_vehicle_win_rates(&dataset.sample, opts.analytics_time_span);
            store_vehicle_win_rates(&mut redis, vehicle_win_rates).await?;
        }
    }

    Ok(last_losses.test)
}

fn adjust_regularization(
    nr_epoch: usize,
    last_losses: &LossPair,
    losses: &LossPair,
    regularization: f64,
    auto_r_bump_chance: Option<f64>,
) -> f64 {
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

    for point in dataset.sample.iter() {
        let factors = model
            .get_factors_mut(point.account_id, remap_tank_id(point.tank_id))
            .await?;

        let prediction = predict_probability(factors.vehicle, factors.account);

        if !point.is_test {
            losses_builder.train.push_sample(prediction, point.is_win);
            let residual_error = if point.is_win {
                1.0 - prediction
            } else {
                -prediction
            };
            make_gradient_descent_step(
                factors.account,
                factors.vehicle,
                residual_error,
                regularization,
                learning_rate,
            );
            model.touch_account(point.account_id);
        } else {
            losses_builder.test.push_sample(prediction, point.is_win);
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
