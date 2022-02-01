use anyhow::Context;
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands};
use tracing::{info, instrument};

use crate::aggregator::models::Analytics;
use crate::wargaming::tank_id::TankId;

pub const UPDATED_AT_KEY: &str = "aggregator::updated_at";
const ANALYTICS_KEY: &str = "analytics::ru";
const CHARTS_KEY: &str = "analytics::ru::charts";

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

#[instrument(level = "info", skip_all)]
pub async fn store_charts(
    redis: &mut MultiplexedConnection,
    charts: impl IntoIterator<Item = (TankId, serde_json::Value)>,
) -> crate::Result {
    let mut pipeline = pipe();
    pipeline
        .atomic()
        .del(CHARTS_KEY)
        .ignore()
        .cmd("HSET")
        .arg(CHARTS_KEY);
    for (tank_id, chart) in charts.into_iter() {
        pipeline.arg(tank_id).arg(chart.to_string());
    }
    let (n_charts,): (u32,) = pipeline
        .query_async(redis)
        .await
        .context("failed to store the charts")?;
    info!(n_charts = n_charts, "stored");
    Ok(())
}

#[instrument(level = "info", skip_all, fields(tank_id = tank_id))]
pub async fn retrieve_vehicle_chart(
    redis: &mut MultiplexedConnection,
    tank_id: TankId,
) -> crate::Result<Option<String>> {
    redis
        .hget(CHARTS_KEY, tank_id)
        .await
        .with_context(|| format!("failed to retrieve a chart for #{}", tank_id))
}
