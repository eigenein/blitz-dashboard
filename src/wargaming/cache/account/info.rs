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
    const TTL_SECS: usize = 60;

    pub fn new(api: WargamingApi, redis: RedisConnection) -> Self {
        Self { api, redis }
    }

    pub async fn get(&self, account_id: i32) -> crate::Result<AccountInfo> {
        let mut redis = self.redis.clone();

        if let Some(blob) = redis
            .get::<_, Option<Bytes>>(Self::cache_key(account_id))
            .await?
        {
            log::debug!("Cache hit on account #{} info.", account_id,);
            return Ok(rmp_serde::from_read_ref(&blob)?);
        }

        let account_info = self
            .api
            .get_account_info(&[account_id])
            .await?
            .remove(&account_id.to_string())
            .flatten()
            .ok_or_else(|| anyhow!("account #{} not found", account_id))?;
        self.put(&account_info).await?;
        Ok(account_info)
    }

    pub async fn put(&self, account_info: &AccountInfo) -> crate::Result {
        self.redis
            .clone()
            .set_ex(
                Self::cache_key(account_info.base.id),
                rmp_serde::to_vec(&account_info)?,
                Self::TTL_SECS,
            )
            .await?;
        Ok(())
    }

    fn cache_key(account_id: i32) -> String {
        format!("account::info::{}", account_id)
    }
}
