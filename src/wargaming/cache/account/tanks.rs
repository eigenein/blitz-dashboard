use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::RedisKey;
use futures::future::try_join;
use mongodb::bson;
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
    const EXPIRE: Option<Expiration> = Some(Expiration::EX(30));

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
        let snapshots = match self.redis.get::<Option<Vec<u8>>, _>(&cache_key).await? {
            Some(blob) => {
                debug!(account_id, "cache hit");
                let blob = decompress(&blob).await?;
                let mut root = bson::from_slice::<bson::Document>(&blob)
                    .context("failed to deserialize the tanks cache")?;
                bson::from_bson(root.remove("root").ok_or_else(|| anyhow!("no root"))?)?
            }
            None => {
                let (statistics, achievements) = {
                    let get_statistics = self.api.get_tanks_stats(realm, account_id);
                    let get_achievements = self.api.get_tanks_achievements(realm, account_id);
                    try_join(get_statistics, get_achievements).await?
                };
                let snapshots =
                    database::TankSnapshot::from_vec(realm, account_id, statistics, achievements);
                let blob = bson::to_vec(&bson::doc! {"root": bson::to_bson(&snapshots)?})?;
                let blob = compress(&blob).await?;
                debug!(account_id, n_bytes = blob.len(), "set cache");
                self.redis
                    .set(&cache_key, blob.as_slice(), Self::EXPIRE, None, false)
                    .await?;
                snapshots
            }
        };
        Ok(snapshots
            .into_iter()
            .map(|snapshot| (snapshot.tank_id, snapshot))
            .collect())
    }

    #[inline]
    fn cache_key(realm: Realm, account_id: AccountId) -> RedisKey {
        RedisKey::from(format!("cache:6:a:t:{}:{}", realm.to_str(), account_id))
    }
}
