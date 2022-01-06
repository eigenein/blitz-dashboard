use anyhow::Context;
use redis::aio::MultiplexedConnection;
use redis::pipe;
use tracing::instrument;

use crate::math::statistics::ConfidenceInterval;
use crate::wargaming::tank_id::TankId;

const LIVE_VEHICLE_WIN_RATES: &str = "trainer::vehicles::win_rates::ru";
const LIVE_VEHICLE_WIN_RATE_MARGINS: &str = "trainer::vehicles::win_rates::margins::ru";

type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

#[instrument(skip_all, fields(n_vehicles = win_rates.len()))]
pub async fn store_vehicle_win_rates(
    redis: &mut MultiplexedConnection,
    win_rates: HashMap<TankId, ConfidenceInterval>,
) -> crate::Result {
    let mut pipeline = pipe();

    pipeline.cmd("HMSET");
    pipeline.arg(LIVE_VEHICLE_WIN_RATES);
    for (tank_id, win_rate) in win_rates.iter() {
        pipeline.arg(tank_id).arg(win_rate.mean);
    }
    pipeline.ignore();

    pipeline.cmd("HMSET");
    pipeline.arg(LIVE_VEHICLE_WIN_RATE_MARGINS);
    for (tank_id, win_rate) in win_rates.into_iter() {
        pipeline.arg(tank_id).arg(win_rate.margin);
    }
    pipeline.ignore();

    pipeline
        .query_async(redis)
        .await
        .context("failed to store the vehicle win rates")
}

#[instrument(level = "debug", skip_all)]
pub async fn retrieve_vehicle_win_rates(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<TankId, ConfidenceInterval>> {
    let (means, mut margins): (HashMap<TankId, f64>, HashMap<TankId, f64>) = pipe()
        .hgetall(LIVE_VEHICLE_WIN_RATES)
        .hgetall(LIVE_VEHICLE_WIN_RATE_MARGINS)
        .query_async(redis)
        .await
        .context("failed to retrieve vehicle win rates")?;
    let win_rates = means
        .into_iter()
        .filter_map(|(tank_id, mean)| {
            margins
                .remove(&tank_id)
                .map(|margin| (tank_id, mean, margin))
        })
        .map(|(tank_id, mean, margin)| (tank_id, ConfidenceInterval { mean, margin }))
        .collect();

    Ok(win_rates)
}
