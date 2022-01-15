use anyhow::Context;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tracing::instrument;

use crate::aggregator::models::Analytics;

pub const UPDATED_AT_KEY: &str = "aggregator::updated_at";
const ANALYTICS_KEY: &str = "analytics::ru";

#[instrument(
    skip_all,
    fields(
        n_time_spans = analytics.time_spans.len(),
        n_vehicles = analytics.win_rates.len(),
    ),
)]
pub async fn store_analytics(
    redis: &mut MultiplexedConnection,
    analytics: &Analytics,
) -> crate::Result {
    redis
        .set(ANALYTICS_KEY, rmp_serde::to_vec_named(analytics)?)
        .await
        .context("failed to store the analytics")
}

#[instrument(level = "debug", skip_all)]
pub async fn retrieve_analytics(redis: &mut MultiplexedConnection) -> crate::Result<Analytics> {
    let blob: Vec<u8> = redis.get(ANALYTICS_KEY).await?;
    Ok(rmp_serde::from_read_ref(&blob)?)
}
