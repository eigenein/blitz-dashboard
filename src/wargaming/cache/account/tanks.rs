use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::RedisKey;
use futures::future::try_join;
use tracing::{debug, instrument};

use crate::database;
use crate::helpers::compression::{compress, decompress};
use crate::prelude::*;
use crate::wargaming::{AccountId, Realm, TankId, WargamingApi};

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

    #[instrument(skip_all, fields(realm = ?realm, account_id = account_id))]
    pub async fn get(
        &self,
        realm: Realm,
        account_id: AccountId,
    ) -> Result<AHashMap<TankId, database::TankSnapshot>> {
        let cache_key = Self::cache_key(realm, account_id);

        if let Some(blob) = self.redis.get::<Option<Vec<u8>>, _>(&cache_key).await? {
            debug!(account_id, "cache hit");
            return Ok(rmp_serde::from_slice(&decompress(&blob).await?)?);
        }

        let (statistics, achievements) = {
            let get_statistics = self.api.get_tanks_stats(realm, account_id);
            let get_achievements = self.api.get_tanks_achievements(realm, account_id);
            try_join(get_statistics, get_achievements).await?
        };
        let snapshots =
            database::TankSnapshot::from_vec(realm, account_id, statistics, achievements)
                .into_iter()
                .map(|snapshot| (snapshot.tank_id, snapshot))
                .collect();
        let blob = compress(&rmp_serde::to_vec(&snapshots)?).await?;
        debug!(account_id, n_bytes = blob.len(), "set cache");
        self.redis
            .set(&cache_key, blob.as_slice(), Self::EXPIRE, None, false)
            .await?;
        Ok(snapshots)
    }

    #[inline]
    fn cache_key(realm: Realm, account_id: AccountId) -> RedisKey {
        RedisKey::from(format!("cache:4:a:t:{}:{}", realm.to_str(), account_id))
    }
}
