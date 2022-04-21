use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tracing::instrument;

use crate::models::AccountInfo;
use crate::wargaming::WargamingApi;

pub struct AccountInfoCache {
    api: WargamingApi,
    redis: MultiplexedConnection,
}

impl AccountInfoCache {
    const TTL_SECS: usize = 60;

    pub fn new(api: WargamingApi, redis: MultiplexedConnection) -> Self {
        Self { api, redis }
    }

    #[instrument(level = "debug", skip_all, fields(account_id = account_id))]
    pub async fn get(&self, account_id: i32) -> crate::Result<Option<AccountInfo>> {
        let mut redis = self.redis.clone();

        if let Some(blob) = redis
            .get::<_, Option<Vec<u8>>>(Self::cache_key(account_id))
            .await?
        {
            tracing::debug!(account_id = account_id, "cache hit");
            return Ok(rmp_serde::from_slice(&blob)?);
        }

        let account_info = self
            .api
            .get_account_info(&[account_id])
            .await?
            .remove(&account_id.to_string())
            .flatten();
        if let Some(account_info) = &account_info {
            self.put(account_info).await?;
        }
        Ok(account_info)
    }

    #[instrument(level = "debug", skip_all, fields(account_id = account_info.base.id))]
    pub async fn put(&self, account_info: &AccountInfo) -> crate::Result {
        let blob = rmp_serde::to_vec(&account_info)?;
        tracing::debug!(
            account_id = account_info.base.id,
            n_bytes = blob.len(),
            "caching",
        );
        self.redis
            .clone()
            .set_ex(Self::cache_key(account_info.base.id), blob, Self::TTL_SECS)
            .await?;
        Ok(())
    }

    fn cache_key(account_id: i32) -> String {
        format!("a::i::ru::{}", account_id)
    }
}
