use fred::pool::RedisPool;
use fred::prelude::*;
use fred::types::RedisKey;
use mongodb::bson;
use tracing::{debug, instrument};

use crate::prelude::*;
use crate::wargaming::models::AccountInfo;
use crate::wargaming::{AccountId, Realm, WargamingApi};

#[derive(Clone)]
pub struct AccountInfoCache {
    api: WargamingApi,
    redis: RedisPool,
}

impl AccountInfoCache {
    const EXPIRE: Option<Expiration> = Some(Expiration::EX(30));

    pub const fn new(api: WargamingApi, redis: RedisPool) -> Self {
        Self { api, redis }
    }

    #[instrument(skip_all, fields(realm = ?realm, account_id = account_id))]
    pub async fn get(&self, realm: Realm, account_id: AccountId) -> Result<Option<AccountInfo>> {
        if let Some(blob) = self
            .redis
            .get::<Option<Vec<u8>>, _>(Self::cache_key(realm, account_id))
            .await?
        {
            debug!(account_id = account_id, "cache hit");
            return Ok(bson::from_slice(&blob)?);
        }

        let account_info = self
            .api
            .get_account_info(realm, &[account_id])
            .await?
            .remove(&account_id.to_string())
            .flatten();
        if let Some(account_info) = &account_info {
            self.put(realm, account_info).await?;
        }
        Ok(account_info)
    }

    #[instrument(skip_all, fields(realm = ?realm, account_id = account_info.id))]
    pub async fn put(&self, realm: Realm, account_info: &AccountInfo) -> Result {
        let blob = bson::to_vec(&account_info)?;
        debug!(account_id = account_info.id, n_bytes = blob.len(), "set cache");
        self.redis
            .set(
                Self::cache_key(realm, account_info.id),
                blob.as_slice(),
                Self::EXPIRE,
                None,
                false,
            )
            .await?;
        Ok(())
    }

    #[inline]
    fn cache_key(realm: Realm, account_id: AccountId) -> RedisKey {
        RedisKey::from(format!("cache:3:a:i:{}:{}", realm.to_str(), account_id))
    }
}
