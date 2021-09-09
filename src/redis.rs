use anyhow::Context;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

pub async fn open(uri: &str) -> crate::Result<ConnectionManager> {
    Ok(redis::Client::open(uri)
        .context("failed to parse Redis URI")?
        .get_tokio_connection_manager()
        .await
        .context("failed to connect to Redis")?)
}

pub async fn get_vehicle_factors(
    redis: &mut ConnectionManager,
    tank_id: i32,
) -> crate::Result<Vec<f64>> {
    let value: Option<Vec<u8>> = redis.get(&get_vehicle_factors_key(tank_id)).await?;
    match value {
        Some(value) => Ok(rmp_serde::from_read_ref(&value)?),
        None => Ok(Vec::new()),
    }
}

pub async fn set_vehicle_factors(
    redis: &mut ConnectionManager,
    tank_id: i32,
    factors: &[f64],
) -> crate::Result {
    // TODO: remove when `cf::vehicles` is populated.
    redis
        .set(
            &get_vehicle_factors_key(tank_id),
            rmp_serde::to_vec(factors)?,
        )
        .await?;

    redis
        .hset("cf::vehicles", tank_id, rmp_serde::to_vec(factors)?)
        .await?;
    Ok(())
}

#[deprecated]
fn get_vehicle_factors_key(tank_id: i32) -> String {
    format!("t:{}:factors", tank_id)
}
