use std::error::Error as StdError;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use moka::future::{Cache, CacheBuilder};

use crate::models::AccountInfo;
use crate::time::from_minutes;
use crate::wargaming::WargamingApi;

pub struct AccountInfoCache {
    api: WargamingApi,
    cache: Cache<i32, Arc<AccountInfo>>,
}

impl AccountInfoCache {
    pub fn new(api: WargamingApi) -> Self {
        Self {
            api,
            cache: CacheBuilder::new(1_000)
                .time_to_live(from_minutes(1))
                .build(),
        }
    }

    pub async fn get(&self, account_id: i32) -> crate::Result<Arc<AccountInfo>> {
        self.cache
            .get_or_try_insert_with(account_id, async {
                Ok(Arc::new(
                    self.api
                        .get_account_info(&[account_id])
                        .await
                        .map_err(Into::<Box<dyn StdError + Send + Sync + 'static>>::into)?
                        .remove(&account_id.to_string())
                        .flatten()
                        .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
                ))
            })
            .await
            .map_err(|error| anyhow::anyhow!(error))
            .with_context(|| format!("failed to access the cache for account #{}", account_id))
    }
}
