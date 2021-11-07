use chrono::{DateTime, Utc};
use futures::future::try_join;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::helpers::{compress_to_vec, decompress_to_vec};
use crate::models::{merge_tanks, Tank};
use crate::wargaming::WargamingApi;

#[derive(Clone)]
pub struct AccountTanksCache {
    api: WargamingApi,
    redis: MultiplexedConnection,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    tanks: Vec<Tank>,

    #[serde(with = "chrono::serde::ts_seconds")]
    last_battle_time: DateTime<Utc>,
}

impl AccountTanksCache {
    const TTL_SECS: usize = 120 * 60;

    pub fn new(api: WargamingApi, redis: MultiplexedConnection) -> Self {
        Self { api, redis }
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get(
        &self,
        account_id: i32,
        last_battle_time: DateTime<Utc>,
    ) -> crate::Result<Vec<Tank>> {
        let mut redis = self.redis.clone();
        let cache_key = Self::cache_key(account_id);

        if let Some(blob) = redis.get::<_, Option<Vec<u8>>>(&cache_key).await? {
            let entry: Entry = rmp_serde::from_read_ref(&decompress_to_vec(blob).await?)?;
            if entry.last_battle_time == last_battle_time {
                tracing::debug!(account_id = account_id, "cache hit");
                return Ok(entry.tanks);
            }
        }

        let (statistics, achievements) = {
            let get_statistics = self.api.get_tanks_stats(account_id);
            let get_achievements = self.api.get_tanks_achievements(account_id);
            try_join(get_statistics, get_achievements).await?
        };
        let entry = Entry {
            last_battle_time,
            tanks: merge_tanks(account_id, statistics, achievements),
        };
        let blob = compress_to_vec(rmp_serde::to_vec(&entry)?, 1).await?;
        tracing::debug!(account_id = account_id, size = blob.len(), "set cache");
        redis.set_ex(&cache_key, blob, Self::TTL_SECS).await?;
        Ok(entry.tanks)
    }

    fn cache_key(account_id: i32) -> String {
        format!("a::t::ru::{}", account_id)
    }
}
