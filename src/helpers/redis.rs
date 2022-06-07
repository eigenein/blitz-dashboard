use anyhow::Context;
use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::PerformanceConfig;

use crate::prelude::*;

#[instrument(level = "info")]
pub async fn connect(url: &str, pool_size: usize) -> Result<RedisPool> {
    let mut config = RedisConfig::from_url(url)?;
    config.blocking = Blocking::Error;
    config.tracing = true;
    config.performance = PerformanceConfig {
        pipeline: false,
        ..Default::default()
    };

    let pool = RedisPool::new(config, pool_size)?;
    pool.connect(None);
    pool.wait_for_connect()
        .await
        .context("failed to connect to Redis")?;
    pool.client_setname(env!("CARGO_BIN_NAME")).await?;
    info!("connected");
    Ok(pool)
}
