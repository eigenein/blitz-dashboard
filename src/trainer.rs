use humantime::format_duration;
use redis::aio::MultiplexedConnection;
use tokio::time::interval;

use crate::opts::TrainerOpts;
use crate::trainer::dataset::{calculate_vehicle_win_rates, Dataset};
use crate::trainer::model::store_vehicle_win_rates;

pub mod dataset;
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
    run_epochs(redis, opts, dataset).await?;

    Ok(())
}

#[tracing::instrument(
    skip_all,
    fields(
        flush_interval = %format_duration(opts.interval),
    ),
)]
async fn run_epochs(
    mut redis: MultiplexedConnection,
    opts: TrainerOpts,
    mut dataset: Dataset,
) -> crate::Result<f64> {
    let mut interval = interval(opts.interval);

    loop {
        interval.tick().await;

        let vehicle_win_rates = calculate_vehicle_win_rates(&dataset.sample, opts.time_span);
        store_vehicle_win_rates(&mut redis, vehicle_win_rates).await?;

        dataset.refresh().await?;
    }
}
