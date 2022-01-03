use futures::future::try_join;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tracing::instrument;

use crate::helpers::{compress_to_vec, decompress_to_vec};
use crate::models::{merge_tanks, Tank};
use crate::wargaming::WargamingApi;

#[derive(Clone)]
pub struct AccountTanksCache {
    api: WargamingApi,
    redis: MultiplexedConnection,
}

impl AccountTanksCache {
    const TTL_SECS: usize = 60;

    pub fn new(api: WargamingApi, redis: MultiplexedConnection) -> Self {
        Self { api, redis }
    }

    #[instrument(level = "debug", skip_all, fields(account_id = account_id))]
    pub async fn get(&self, account_id: i32) -> crate::Result<Vec<Tank>> {
        let mut redis = self.redis.clone();
        let cache_key = Self::cache_key(account_id);

        if let Some(blob) = redis.get::<_, Option<Vec<u8>>>(&cache_key).await? {
            tracing::debug!(account_id = account_id, "cache hit");
            return Ok(rmp_serde::from_read_ref(&decompress_to_vec(blob).await?)?);
        }

        let (statistics, achievements) = {
            let get_statistics = self.api.get_tanks_stats(account_id);
            let get_achievements = self.api.get_tanks_achievements(account_id);
            try_join(get_statistics, get_achievements).await?
        };
        let tanks = merge_tanks(account_id, statistics, achievements);
        let blob = compress_to_vec(rmp_serde::to_vec(&tanks)?, 1).await?;
        tracing::debug!(account_id = account_id, size = blob.len(), "set cache");
        redis.set_ex(&cache_key, blob, Self::TTL_SECS).await?;
        Ok(tanks)
    }

    fn cache_key(account_id: i32) -> String {
        format!("a::t2::ru::{}", account_id)
    }
}
