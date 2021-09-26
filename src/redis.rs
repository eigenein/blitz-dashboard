use anyhow::Context;
use redis::aio::ConnectionManager as Connection;

pub async fn open(uri: &str) -> crate::Result<Connection> {
    Ok(redis::Client::open(uri)
        .context("failed to parse Redis URI")?
        .get_tokio_connection_manager()
        .await
        .context("failed to connect to Redis")?)
}
