use async_std::sync::Arc;
use moka::future::{Cache, CacheBuilder};

use crate::models::AccountInfo;
use crate::time::*;
use crate::wargaming::WargamingApi;

pub struct AccountSearchCache {
    api: WargamingApi,
    id_cache: Cache<String, Arc<Vec<i32>>>,
    info_cache: Cache<String, Arc<Vec<AccountInfo>>>,
}

impl AccountSearchCache {
    pub fn new(api: WargamingApi) -> Self {
        Self {
            api,
            id_cache: CacheBuilder::new(1_000).time_to_live(DAY).build(),
            info_cache: CacheBuilder::new(1_000).time_to_live(FIVE_MINUTES).build(),
        }
    }

    #[allow(clippy::ptr_arg)]
    pub async fn get(&self, query: &String) -> crate::Result<Arc<Vec<AccountInfo>>> {
        match self.info_cache.get(query) {
            // Check if we already have up-to-date search results.
            Some(infos) => Ok(infos),

            None => {
                let account_ids = match self.id_cache.get(query) {
                    // Check if we already have account IDs for this query.
                    Some(account_ids) => account_ids,

                    None => {
                        let account_ids: Vec<i32> = self
                            .api
                            .search_accounts(query)
                            .await?
                            .iter()
                            .map(|account| account.id)
                            .collect();
                        let account_ids = Arc::new(account_ids);
                        self.id_cache
                            .insert(query.clone(), account_ids.clone())
                            .await;
                        account_ids
                    }
                };

                let account_infos: Vec<AccountInfo> = self
                    .api
                    .get_account_info(&account_ids)
                    .await?
                    .into_iter()
                    .filter_map(|(_, info)| info)
                    .collect();
                let account_infos = Arc::new(account_infos);
                self.info_cache
                    .insert(query.clone(), account_infos.clone())
                    .await;
                Ok(account_infos)
            }
        }
    }
}
