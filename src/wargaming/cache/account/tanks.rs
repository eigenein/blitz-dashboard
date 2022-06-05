use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::RedisKey;
use futures::future::try_join;
use tracing::{debug, instrument};

use crate::models::{merge_tanks, Tank};
use crate::prelude::*;
use crate::wargaming::WargamingApi;

#[derive(Clone)]
pub struct AccountTanksCache {
    api: WargamingApi,
    redis: RedisPool,
}

impl AccountTanksCache {
    const EXPIRE: Option<Expiration> = Some(Expiration::EX(60));

    pub fn new(api: WargamingApi, redis: RedisPool) -> Self {
        Self { api, redis }
    }

    #[instrument(skip_all, fields(account_id = account_id))]
    pub async fn get(&self, account_id: i32) -> Result<Vec<Tank>> {
        let cache_key = Self::cache_key(account_id);

        if let Some(blob) = self.redis.get::<Option<Vec<u8>>, _>(&cache_key).await? {
            debug!(account_id, "cache hit");
            return Ok(rmp_serde::from_slice(&blob)?);
        }

        let (statistics, achievements) = {
            let get_statistics = self.api.get_tanks_stats(account_id);
            let get_achievements = self.api.get_tanks_achievements(account_id);
            try_join(get_statistics, get_achievements).await?
        };
        let tanks = merge_tanks(account_id, statistics, achievements);
        let blob = rmp_serde::to_vec(&tanks)?;
        debug!(account_id, size = blob.len(), "set cache");
        self.redis
            .set(&cache_key, blob.as_slice(), Self::EXPIRE, None, false)
            .await?;
        Ok(tanks)
    }

    #[inline]
    fn cache_key(account_id: i32) -> RedisKey {
        RedisKey::from(format!("cache:a:t2:ru:{}", account_id))
    }
}
