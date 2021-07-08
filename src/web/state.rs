use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use moka::future::{Cache, CacheBuilder};
use sqlx::PgPool;

use crate::database;
use crate::models::{AccountInfo, Nation, Tank, TankType, Vehicle};
use crate::opts::WebOpts;
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: PgPool,
    pub extra_html_headers: String,

    /// Caches search query to accounts IDs, optimises searches for popular accounts.
    search_accounts_ids_cache: Cache<String, Arc<Vec<i32>>>,

    /// Caches search query to search results, expires way sooner but provides more accurate data.
    search_accounts_infos_cache: Cache<String, Arc<Vec<AccountInfo>>>,

    tankopedia: HashMap<i32, Arc<Vehicle>>,
    account_info_cache: Cache<i32, Arc<AccountInfo>>,
    account_tanks_cache: Cache<i32, (DateTime<Utc>, Arc<Vec<Tank>>)>,
    retrieve_count_cache: Cache<RetrieveCount, i64>,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum RetrieveCount {
    Accounts,
    AccountSnapshots,
    TankSnapshots,
}

impl State {
    pub async fn new(api: WargamingApi, database: PgPool, opts: &WebOpts) -> crate::Result<Self> {
        let tankopedia: HashMap<i32, Arc<Vehicle>> = Self::retrieve_tankopedia(&database).await?;

        let state = State {
            api,
            database,
            extra_html_headers: Self::make_extra_html_headers(&opts),
            tankopedia,
            search_accounts_ids_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(86400))
                .build(),
            search_accounts_infos_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(300))
                .build(),
            account_info_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(60))
                .build(),
            account_tanks_cache: CacheBuilder::new(1_000).build(),
            retrieve_count_cache: CacheBuilder::new(1_000)
                .time_to_live(StdDuration::from_secs(300))
                .build(),
        };
        Ok(state)
    }

    pub async fn retrieve_count(&self, key: RetrieveCount) -> crate::Result<i64> {
        match self.retrieve_count_cache.get(&key) {
            Some(count) => Ok(count),
            None => {
                let count = match key {
                    RetrieveCount::Accounts => {
                        database::retrieve_account_count(&self.database).await?
                    }
                    RetrieveCount::AccountSnapshots => {
                        database::retrieve_account_snapshot_count(&self.database).await?
                    }
                    RetrieveCount::TankSnapshots => {
                        database::retrieve_tank_snapshot_count(&self.database).await?
                    }
                };
                self.retrieve_count_cache.insert(key, count).await;
                Ok(count)
            }
        }
    }

    #[allow(clippy::ptr_arg)]
    pub async fn search_accounts(&self, query: &String) -> crate::Result<Arc<Vec<AccountInfo>>> {
        match self.search_accounts_infos_cache.get(query) {
            // Check if we already have up-to-date search results.
            Some(infos) => Ok(infos),

            None => {
                let account_ids = match self.search_accounts_ids_cache.get(query) {
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
                        self.search_accounts_ids_cache
                            .insert(query.clone(), account_ids.clone())
                            .await;
                        account_ids
                    }
                };

                let account_infos: Vec<AccountInfo> = self
                    .api
                    .get_account_info(account_ids.iter())
                    .await?
                    .into_iter()
                    .filter_map(|(_, info)| info)
                    .collect();
                let account_infos = Arc::new(account_infos);
                self.search_accounts_infos_cache
                    .insert(query.clone(), account_infos.clone())
                    .await;
                Ok(account_infos)
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

    pub async fn retrieve_tanks(
        &self,
        account_info: &AccountInfo,
    ) -> crate::Result<Arc<Vec<Tank>>> {
        let account_id = account_info.basic.id;
        match self.account_tanks_cache.get(&account_id) {
            Some((last_battle_time, snapshots))
                if last_battle_time == account_info.basic.last_battle_time =>
            {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots)
            }
            _ => {
                let snapshots = Arc::new(self.api.get_merged_tanks(account_id).await?);
                self.account_tanks_cache
                    .insert(
                        account_id,
                        (account_info.basic.last_battle_time, snapshots.clone()),
                    )
                    .await;
                Ok(snapshots)
            }
        }
    }

    pub fn get_vehicle(&self, tank_id: i32) -> Arc<Vehicle> {
        self.tankopedia
            .get(&tank_id)
            .cloned()
            .unwrap_or_else(|| Self::new_hardcoded_vehicle(tank_id))
    }

    async fn retrieve_tankopedia(database: &PgPool) -> crate::Result<HashMap<i32, Arc<Vehicle>>> {
        let mut tankopedia: HashMap<i32, Arc<Vehicle>> = database::retrieve_vehicles(&database)
            .await?
            .into_iter()
            .map(|vehicle| (vehicle.tank_id, Arc::new(vehicle)))
            .collect();
        tankopedia.insert(
            23057,
            Arc::new(Vehicle {
                tank_id: 23057,
                name: "Kunze Panzer".to_string(),
                tier: 7,
                is_premium: true,
                nation: Nation::Germany,
                type_: TankType::Light,
            }),
        );
        Ok(tankopedia)
    }

    fn new_hardcoded_vehicle(tank_id: i32) -> Arc<Vehicle> {
        Arc::new(Vehicle {
            tank_id,
            name: format!("#{}", tank_id),
            tier: 0,
            is_premium: false,
            type_: TankType::Light, // FIXME
            nation: Nation::Other,
        })
    }

    fn make_extra_html_headers(opts: &WebOpts) -> String {
        let mut extra_html_headers = Vec::new();
        if let Some(counter) = &opts.yandex_metrika {
            extra_html_headers.push(format!(
                r#"<!-- Yandex.Metrika counter --> <script type="text/javascript" > (function(m,e,t,r,i,k,a){{m[i]=m[i]||function(){{(m[i].a=m[i].a||[]).push(arguments)}}; m[i].l=1*new Date();k=e.createElement(t),a=e.getElementsByTagName(t)[0],k.async=1,k.src=r,a.parentNode.insertBefore(k,a)}}) (window, document, "script", "https://mc.yandex.ru/metrika/tag.js", "ym"); ym({}, "init", {{ clickmap:true, trackLinks:true, accurateTrackBounce:true, trackHash:true }}); </script> <noscript><div><img src="https://mc.yandex.ru/watch/{}" style="position:absolute; left:-9999px;" alt=""/></div></noscript> <!-- /Yandex.Metrika counter -->"#,
                counter, counter,
            ));
        };
        if let Some(measurement_id) = &opts.gtag {
            extra_html_headers.push(format!(
                r#"<!-- Global site tag (gtag.js) - Google Analytics --> <script async src="https://www.googletagmanager.com/gtag/js?id=G-S1HXCH4JPZ"></script> <script>window.dataLayer = window.dataLayer || []; function gtag(){{dataLayer.push(arguments);}} gtag('js', new Date()); gtag('config', '{}'); </script>"#,
                measurement_id,
            ));
        };
        extra_html_headers.join("")
    }
}
