use anyhow::anyhow;
use async_std::sync::Arc;
use moka::future::{Cache, CacheBuilder};

use crate::models::AccountInfo;
use crate::time::*;
use crate::wargaming::WargamingApi;

pub struct AccountInfoCache {
    api: WargamingApi,
    cache: Cache<i32, Arc<AccountInfo>>,
}

impl AccountInfoCache {
    pub fn new(api: WargamingApi) -> Self {
        Self {
            api,
            cache: CacheBuilder::new(1_000).time_to_live(MINUTE).build(),
        }
    }

    pub async fn get(&self, account_id: i32) -> crate::Result<Arc<AccountInfo>> {
        match self.cache.get(&account_id) {
            Some(account_info) => {
                log::debug!("Cache hit on account #{} info.", account_id);
                Ok(account_info)
            }
            None => {
                let account_info = Arc::new(
                    self.api
                        .get_account_info(&[account_id])
                        .await?
                        .remove(&account_id.to_string())
                        .flatten()
                        .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
                );
                self.cache.insert(account_id, account_info.clone()).await;
                Ok(account_info)
            }
        }
    }
}
