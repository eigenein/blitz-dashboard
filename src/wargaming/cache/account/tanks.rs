use bytes::Bytes;
use chrono::{DateTime, Utc};
use redis::aio::ConnectionManager as RedisConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::models::{merge_tanks, Tank};
use crate::wargaming::WargamingApi;

pub struct AccountTanksCache {
    api: WargamingApi,
    redis: RedisConnection,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    tanks: Vec<Tank>,

    #[serde(with = "chrono::serde::ts_seconds")]
    last_battle_time: DateTime<Utc>,
}

impl AccountTanksCache {
    const TTL_SECS: usize = 15 * 60;

    pub fn new(api: WargamingApi, redis: RedisConnection) -> Self {
        Self { api, redis }
    }

    pub async fn get(
        &self,
        account_id: i32,
        last_battle_time: DateTime<Utc>,
    ) -> crate::Result<Vec<Tank>> {
        let mut redis = self.redis.clone();
        let cache_key = Self::cache_key(account_id);

        if let Some(blob) = redis.get::<_, Option<Bytes>>(&cache_key).await? {
            let entry: Entry = rmp_serde::from_read_ref(&blob)?;
            if entry.last_battle_time == last_battle_time {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                return Ok(entry.tanks);
            }
        }

        let statistics = self.api.get_tanks_stats(account_id).await?;
        let achievements = self.api.get_tanks_achievements(account_id).await?;
        let entry = Entry {
            last_battle_time,
            tanks: merge_tanks(account_id, statistics, achievements),
        };
        let blob = rmp_serde::to_vec(&entry)?;
        log::debug!("Caching account #{} tanks: {} B.", account_id, blob.len());
        redis.set_ex(&cache_key, blob, Self::TTL_SECS).await?;
        Ok(entry.tanks)
    }

    fn cache_key(account_id: i32) -> String {
        format!("a:t:ru:{}", account_id)
    }
}
