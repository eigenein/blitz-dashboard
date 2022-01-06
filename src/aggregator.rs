use tokio::time::interval;

use crate::aggregator::redis::store_vehicle_win_rates;
use crate::aggregator::stream::{calculate_vehicle_win_rates, Stream};
use crate::opts::AggregateOpts;

pub mod redis;
pub mod stream;
pub mod stream_entry;

#[tracing::instrument(
    skip_all,
    fields(
        time_span = opts.time_span.to_string().as_str(),
    ),
)]
pub async fn run(opts: AggregateOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "aggregator"));

    let mut redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let mut stream = Stream::read(redis.clone(), opts.time_span).await?;
    let mut interval = interval(opts.interval);

    loop {
        interval.tick().await;

        let vehicle_win_rates = calculate_vehicle_win_rates(&stream.sample, opts.time_span);
        store_vehicle_win_rates(&mut redis, vehicle_win_rates).await?;

        stream.refresh().await?;
    }
}
