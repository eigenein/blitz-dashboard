use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::anyhow;
use async_std::sync::{Mutex, MutexGuard};
use chrono::Utc;
use moka::future::{Cache, CacheBuilder};

use crate::database::Database;
use crate::models::{Account, AccountInfo, Nation, TankSnapshot, TankType, Vehicle};
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    database: Arc<Mutex<Database>>,

    search_accounts_cache: Cache<String, Arc<Vec<Account>>>,
    tankopedia_cache: Cache<i32, Arc<Vehicle>>,
    account_info_cache: Cache<i32, Arc<AccountInfo>>,
    account_tanks_cache: Cache<i32, Arc<Vec<TankSnapshot>>>,
}

impl State {
    pub fn new(api: WargamingApi, database: Database) -> Self {
        State {
            api,
            database: Arc::new(Mutex::new(database)),
            search_accounts_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(86400))
                .build(),
            tankopedia_cache: CacheBuilder::new(1_000_000)
                .time_to_live(StdDuration::from_secs(86400))
                .build(),
            account_info_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(60))
                .build(),
            account_tanks_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(60))
                .build(),
        }
    }

    pub async fn query_database<T, C>(&self, callable: C) -> T
    where
        T: Send + 'static,
        C: FnOnce(MutexGuard<'_, Database>) -> T + Send + 'static,
    {
        let database = self.database.clone();
        async_std::task::spawn(async move { callable(database.lock().await) }).await
    }

    #[allow(clippy::ptr_arg)]
    pub async fn search_accounts(&self, query: &String) -> crate::Result<Arc<Vec<Account>>> {
        match self.search_accounts_cache.get(query) {
            Some(accounts) => Ok(accounts),
            None => {
                let accounts = Arc::new(self.api.search_accounts(query).await?);
                self.search_accounts_cache
                    .insert(query.clone(), accounts.clone())
                    .await;
                Ok(accounts)
            }
        }
    }

    pub async fn retrieve_account_info(&self, account_id: i32) -> crate::Result<Arc<AccountInfo>> {
        match self.account_info_cache.get(&account_id) {
            Some(account_info) => {
                log::debug!("Cache hit on account #{} info.", account_id);
                Ok(account_info)
            }
            None => {
                let account_info = Arc::new(
                    self.api
                        .get_account_info([account_id])
                        .await?
                        .remove(&account_id.to_string())
                        .flatten()
                        .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
                );
                self.account_info_cache
                    .insert(account_id, account_info.clone())
                    .await;
                Ok(account_info)
            }
        }
    }

    pub async fn retrieve_tanks(&self, account_id: i32) -> crate::Result<Arc<Vec<TankSnapshot>>> {
        match self.account_tanks_cache.get(&account_id) {
            Some(snapshots) => {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots)
            }
            None => {
                let snapshots = Arc::new(self.api.get_merged_tanks(account_id).await?);
                self.account_tanks_cache
                    .insert(account_id, snapshots.clone())
                    .await;
                Ok(snapshots)
            }
        }
    }

    /// Retrieves cached vehicle information.
    pub async fn get_vehicle(&self, tank_id: i32) -> crate::Result<Arc<Vehicle>> {
        match self.tankopedia_cache.get(&tank_id) {
            Some(vehicle) => Ok(vehicle),
            None => {
                let vehicle = self
                    .query_database(move |database| database.retrieve_vehicle(tank_id))
                    .await?;
                let vehicle =
                    Arc::new(vehicle.unwrap_or_else(|| Self::get_hardcoded_vehicle(tank_id)));
                self.tankopedia_cache.insert(tank_id, vehicle.clone()).await;
                Ok(vehicle)
            }
        }
    }

    fn get_hardcoded_vehicle(tank_id: i32) -> Vehicle {
        match tank_id {
            23057 => Vehicle {
                tank_id,
                name: "Kunze Panzer".to_string(),
                tier: 7,
                is_premium: true,
                nation: Nation::Germany,
                type_: TankType::Light,
                imported_at: Utc::now(),
            },
            _ => Vehicle {
                tank_id,
                name: format!("#{}", tank_id),
                tier: 0,
                is_premium: false,
                type_: TankType::Other,
                imported_at: Utc::now(),
                nation: Nation::Other,
            },
        }
    }
}
