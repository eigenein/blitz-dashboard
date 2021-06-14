use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use async_std::sync::Mutex;
use async_std::task::spawn;
use lru_time_cache::LruCache;

use crate::cached::Cached;
use crate::database::Database;
use crate::logging::log_anyhow;
use crate::models::{Account, AccountInfo, TankSnapshot, Vehicle};
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    api: WargamingApi,
    database: Arc<Mutex<Database>>,

    search_accounts_cache: Cached<String, Vec<Account>>,
    account_info_cache: Cached<i32, AccountInfo>,
    tanks_cache: Cached<i32, Vec<TankSnapshot>>,
    tankopedia_cache: Arc<Mutex<LruCache<i32, Arc<Option<Vehicle>>>>>,
}

pub struct DatabaseStatistics {
    pub account_count: i64,
    pub account_snapshot_count: i64,
    pub tank_snapshot_count: i64,
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
            tankopedia_cache: Arc::new(Mutex::new(LruCache::with_expiry_duration(
                Duration::from_secs(86400),
            ))),
        }
    }

    pub async fn get_database_statistics(&self) -> crate::Result<DatabaseStatistics> {
        let account_count = {
            let database = self.database.clone();
            spawn(async move { database.lock().await.retrieve_account_count() }).await?
        };
        let account_snapshot_count = {
            let database = self.database.clone();
            spawn(async move { database.lock().await.retrieve_account_snapshot_count() }).await?
        };
        let tank_snapshot_count = {
            let database = self.database.clone();
            spawn(async move { database.lock().await.retrieve_tank_snapshot_count() }).await?
        };
        Ok(DatabaseStatistics {
            account_count,
            account_snapshot_count,
            tank_snapshot_count,
        })
    }

    pub async fn search_accounts(&self, query: String) -> crate::Result<Arc<Vec<Account>>> {
        self.search_accounts_cache
            .get(&query, || async {
                Ok(Arc::new(self.api.search_accounts(&query).await?))
            })
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
                        log_anyhow(database.start_transaction().and_then(|tx| {
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

    pub async fn get_tanks(&self, account_id: i32) -> crate::Result<Arc<Vec<TankSnapshot>>> {
        self.tanks_cache
            .get(&account_id, || async {
                let tanks = Arc::new(self.api.get_merged_tanks(account_id).await?);
                {
                    let tanks = tanks.clone();
                    let database = self.database.clone();
                    async_std::task::spawn(async move {
                        let database = database.lock().await;
                        log_anyhow(database.start_transaction().and_then(|tx| {
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

    pub async fn get_vehicle(&self, tank_id: i32) -> crate::Result<Arc<Option<Vehicle>>> {
        let mut cache = self.tankopedia_cache.lock().await;
        let vehicle = match cache.get(&tank_id) {
            Some(vehicle) => vehicle.clone(),
            None => {
                let vehicle = Arc::new(self.database.lock().await.retrieve_vehicle(tank_id)?);
                cache.insert(tank_id, vehicle.clone());
                vehicle
            }
        };
        Ok(vehicle)
    }
}
