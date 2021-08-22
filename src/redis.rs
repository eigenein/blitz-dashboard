use anyhow::Context;
use redis::aio::ConnectionManager;

pub async fn open(uri: &str) -> crate::Result<ConnectionManager> {
    Ok(redis::Client::open(uri)
        .context("failed to parse Redis URI")?
        .get_tokio_connection_manager()
        .await
        .context("failed to connect to Redis")?)
}

pub trait CacheKey {
    fn cache_key(&self) -> String;
}
