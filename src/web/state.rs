use std::sync::Arc;
use std::time::Duration;

use lru_time_cache::LruCache;

use crate::cached::Cached;
use crate::database::Database;
use crate::logging::log_anyhow;
use crate::wargaming::{models, WargamingApi};

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: Database,

    search_accounts_cache: Cached<String, Vec<crate::wargaming::models::Account>>,
    account_info_cache: Cached<i32, crate::wargaming::models::AggregatedAccountInfo>,
}

impl State {
    pub fn new(api: WargamingApi, database: Database) -> Self {
        State {
            api,
            database,
            search_accounts_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(86400),
                1000,
            )),
            account_info_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60),
                1000,
            )),
        }
    }

    /// Cached [`WargamingApi::search_accounts`].
    pub async fn search_accounts(&self, query: String) -> crate::Result<Arc<Vec<models::Account>>> {
        self.search_accounts_cache
            .get(&query, || self.api.search_accounts(&query))
            .await
    }

    pub async fn get_aggregated_account_info(
        &self,
        account_id: i32,
    ) -> crate::Result<Arc<crate::wargaming::models::AggregatedAccountInfo>> {
        self.account_info_cache
            .get(&account_id, || async {
                let account_info =
                    Arc::new(self.api.get_aggregated_account_info(account_id).await?);
                {
                    let database = self.database.clone();
                    let account_info = account_info.clone();
                    async_std::task::spawn(async move {
                        log_anyhow(database.upsert_aggregated_account_info(&account_info).await);
                    });
                }
                Ok(account_info)
            })
            .await
    }
}
