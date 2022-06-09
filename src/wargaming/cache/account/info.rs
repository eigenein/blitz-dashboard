use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::RedisKey;
use tracing::{debug, instrument};

use crate::prelude::*;
use crate::wargaming::models::AccountInfo;
use crate::wargaming::WargamingApi;

pub struct AccountInfoCache {
    api: WargamingApi,
    redis: RedisPool,
}

impl AccountInfoCache {
    const EXPIRE: Option<Expiration> = Some(Expiration::EX(60));

    pub fn new(api: WargamingApi, redis: RedisPool) -> Self {
        Self { api, redis }
    }

    #[instrument(skip_all, fields(account_id = account_id))]
    pub async fn get(&self, account_id: i32) -> Result<Option<AccountInfo>> {
        if let Some(blob) = self
            .redis
            .get::<Option<Vec<u8>>, _>(Self::cache_key(account_id))
            .await?
        {
            debug!(account_id = account_id, "cache hit");
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

    #[instrument(skip_all, fields(account_id = account_info.id))]
    pub async fn put(&self, account_info: &AccountInfo) -> Result {
        let blob = rmp_serde::to_vec(&account_info)?;
        debug!(account_id = account_info.id, n_bytes = blob.len(), "set cache");
        self.redis
            .set(Self::cache_key(account_info.id), blob.as_slice(), Self::EXPIRE, None, false)
            .await?;
        Ok(())
    }

    #[inline]
    fn cache_key(account_id: i32) -> RedisKey {
        RedisKey::from(format!("cache:1:a:i:ru:{}", account_id))
    }
}
