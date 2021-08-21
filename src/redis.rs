use anyhow::Context;
use redis::aio::Connection;

pub async fn open(uri: &str) -> crate::Result<Connection> {
    Ok(redis::Client::open(uri)
        .context("failed to parse Redis URI")?
        .get_async_connection()
        .await
        .context("failed to connect to Redis")?)
}
