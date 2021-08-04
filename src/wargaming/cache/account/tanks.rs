use std::sync::Arc;

use chrono::{DateTime, Utc};
use moka::future::{Cache, CacheBuilder};

use crate::models::{AccountInfo, Tank};
use crate::wargaming::WargamingApi;

pub struct AccountTanksCache {
    api: WargamingApi,
    cache: Cache<i32, Entry>,
}

type Entry = (DateTime<Utc>, Arc<Vec<Tank>>);

impl AccountTanksCache {
    pub fn new(api: WargamingApi) -> Self {
        Self {
            api,
            cache: CacheBuilder::new(1_000).build(),
        }
    }

    pub async fn get(&self, account_info: &AccountInfo) -> crate::Result<Arc<Vec<Tank>>> {
        let account_id = account_info.base.id;
        match self.cache.get(&account_id) {
            Some((last_battle_time, snapshots))
                if last_battle_time == account_info.base.last_battle_time =>
            {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots)
            }
            _ => {
                let snapshots: Arc<Vec<Tank>> =
                    Arc::new(self.api.get_merged_tanks(account_id).await?);
                self.cache
                    .insert(
                        account_id,
                        (account_info.base.last_battle_time, snapshots.clone()),
                    )
                    .await;
                Ok(snapshots)
            }
        }
    }
}
