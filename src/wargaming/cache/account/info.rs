use anyhow::anyhow;
use bytes::Bytes;
use redis::aio::ConnectionManager as RedisConnection;
use redis::AsyncCommands;

use crate::models::AccountInfo;
use crate::wargaming::WargamingApi;

pub struct AccountInfoCache {
    api: WargamingApi,
    redis: RedisConnection,
}

impl AccountInfoCache {
    pub fn new(api: WargamingApi, redis: RedisConnection) -> Self {
        Self { api, redis }
    }

    pub async fn get(&self, account_id: i32) -> crate::Result<AccountInfo> {
        let mut redis = self.redis.clone();
        let cache_key = format!("account::info::{}", account_id);

        if let Some(blob) = redis.get::<_, Option<Bytes>>(&cache_key).await? {
            log::debug!("Cache hit on account #{}.", account_id,);
            return Ok(rmp_serde::from_read_ref(&blob)?);
        }

        let account_info = self
            .api
            .get_account_info(&[account_id])
            .await?
            .remove(&account_id.to_string())
            .flatten()
            .ok_or_else(|| anyhow!("account #{} not found", account_id))?;
        redis
            .set_ex(&cache_key, rmp_serde::to_vec(&account_info)?, 60)
            .await?;
        Ok(account_info)
    }
}
