use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::RedisKey;
use futures::future::try_join;
use tracing::{debug, instrument};

use crate::helpers::compression::{compress, decompress};
use crate::prelude::*;
use crate::wargaming::models::{merge_tanks, Tank};
use crate::wargaming::{AccountId, TankId, WargamingApi};

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
    pub async fn get(&self, account_id: AccountId) -> Result<AHashMap<TankId, Tank>> {
        let cache_key = Self::cache_key(account_id);

        if let Some(blob) = self.redis.get::<Option<Vec<u8>>, _>(&cache_key).await? {
            debug!(account_id, "cache hit");
            return Ok(rmp_serde::from_slice(&decompress(&blob).await?)?);
        }

        let (statistics, achievements) = {
            let get_statistics = self.api.get_tanks_stats(account_id);
            let get_achievements = self.api.get_tanks_achievements(account_id);
            try_join(get_statistics, get_achievements).await?
        };
        let tanks = merge_tanks(account_id, statistics, achievements);
        let blob = compress(&rmp_serde::to_vec(&tanks)?).await?;
        debug!(account_id, n_bytes = blob.len(), "set cache");
        self.redis
            .set(&cache_key, blob.as_slice(), Self::EXPIRE, None, false)
            .await?;
        Ok(tanks)
    }

    #[inline]
    fn cache_key(account_id: AccountId) -> RedisKey {
        RedisKey::from(format!("cache:3:a:t:ru:{}", account_id))
    }
}
