use std::sync::Arc;
use std::time::Duration;

use lru_time_cache::LruCache;

use crate::cached::Cached;
use crate::database::Database;
use crate::logging::log_anyhow;
use crate::models::{Account, AccountInfo, Tank};
use crate::wargaming::WargamingApi;
use anyhow::anyhow;
use async_std::sync::Mutex;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: Arc<Mutex<Database>>,

    search_accounts_cache: Cached<String, Vec<Account>>,
    account_info_cache: Cached<i32, AccountInfo>,
    tanks_cache: Cached<i32, Vec<Tank>>,
}

impl State {
    pub fn new(api: WargamingApi, database: Database) -> Self {
        State {
            api,
            database: Arc::new(Mutex::new(database)),
            search_accounts_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(86400),
                1000,
            )),
            account_info_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60),
                10000,
            )),
            tanks_cache: Cached::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60),
                1000,
            )),
        }
    }

    /// Cached [`WargamingApi::search_accounts`].
    pub async fn search_accounts(&self, query: String) -> crate::Result<Arc<Vec<Account>>> {
        self.search_accounts_cache
            .get(&query, || self.api.search_accounts(&query))
            .await
    }

    pub async fn get_account_info(&self, account_id: i32) -> crate::Result<Arc<AccountInfo>> {
        self.account_info_cache
            .get(&account_id, || async {
                let account_info = Arc::new(
                    self.api
                        .get_account_info(account_id)
                        .await?
                        .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
                );
                {
                    let account_info = account_info.clone();
                    let database = self.database.clone();
                    async_std::task::spawn(async move {
                        let database = database.lock().await;
                        log_anyhow(database.transaction().and_then(|tx| {
                            database.upsert_account(&account_info.basic)?;
                            database.upsert_account_snapshot(&account_info)?;
                            tx.commit()?;
                            Ok(())
                        }));
                    });
                }
                Ok(account_info)
            })
            .await
    }

    pub async fn get_tanks(&self, account_id: i32) -> crate::Result<Arc<Vec<Tank>>> {
        self.tanks_cache
            .get(&account_id, || async {
                let tanks = Arc::new(self.api.get_merged_tanks(account_id).await?);
                {
                    let tanks = tanks.clone();
                    let database = self.database.clone();
                    async_std::task::spawn(async move {
                        let database = database.lock().await;
                        log_anyhow(database.transaction().and_then(|tx| {
                            database.upsert_tanks(&tanks)?;
                            tx.commit()?;
                            Ok(())
                        }));
                    });
                }
                Ok(tanks)
            })
            .await
    }
}
