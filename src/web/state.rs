use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use maud::PreEscaped;
use moka::future::{Cache, CacheBuilder};
use sqlx::PgPool;

use crate::database;
use crate::models::{AccountInfo, Nation, Tank, TankType, Vehicle};
use crate::opts::Opts;
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: PgPool,
    pub tracking_code: PreEscaped<String>,

    tankopedia: HashMap<i32, Vehicle>,

    /// Caches search query to accounts IDs, optimises searches for popular accounts.
    search_accounts_ids_cache: Cache<String, Arc<Vec<i32>>>,

    /// Caches search query to search results, expires way sooner but provides more accurate data.
    search_accounts_infos_cache: Cache<String, Arc<Vec<AccountInfo>>>,

    account_info_cache: Cache<i32, Arc<AccountInfo>>,
    account_tanks_cache: Cache<i32, (DateTime<Utc>, Arc<Vec<Tank>>)>,
    account_count_cache: Cache<(), i64>,
    account_snapshot_count_cache: Cache<(), i64>,
    tank_snapshot_count_cache: Cache<(), i64>,
    crawler_lag_cache: Cache<(), Duration>,
}

impl State {
    pub async fn new(api: WargamingApi, database: PgPool, opts: &Opts) -> crate::Result<Self> {
        const DAY: StdDuration = StdDuration::from_secs(86400);
        const MINUTE: StdDuration = StdDuration::from_secs(60);
        const FIVE_MINUTES: StdDuration = StdDuration::from_secs(300);

        let mut tankopedia = api.get_tankopedia().await?;
        tankopedia.insert(
            23057,
            Vehicle {
                tank_id: 23057,
                name: "Kunze Panzer".to_string(),
                tier: 7,
                is_premium: true,
                nation: Nation::Germany,
                type_: TankType::Light,
            },
        );

        let state = State {
            api,
            database,
            tankopedia,
            tracking_code: Self::make_tracking_code(&opts),
            search_accounts_ids_cache: CacheBuilder::new(1_000).time_to_live(DAY).build(),
            search_accounts_infos_cache: CacheBuilder::new(1_000)
                .time_to_live(FIVE_MINUTES)
                .build(),
            account_info_cache: CacheBuilder::new(1_000).time_to_live(MINUTE).build(),
            account_tanks_cache: CacheBuilder::new(1_000).build(),
            account_count_cache: CacheBuilder::new(1).time_to_live(FIVE_MINUTES).build(),
            account_snapshot_count_cache: CacheBuilder::new(1).time_to_live(FIVE_MINUTES).build(),
            tank_snapshot_count_cache: CacheBuilder::new(1).time_to_live(FIVE_MINUTES).build(),
            crawler_lag_cache: CacheBuilder::new(1).time_to_live(FIVE_MINUTES).build(),
        };
        Ok(state)
    }

    pub async fn retrieve_account_count(&self) -> crate::Result<i64> {
        match self.account_count_cache.get(&()) {
            Some(count) => Ok(count),
            None => {
                let count = database::retrieve_account_count(&self.database).await?;
                self.account_count_cache.insert((), count).await;
                Ok(count)
            }
        }
    }

    pub async fn retrieve_account_snapshot_count(&self) -> crate::Result<i64> {
        match self.account_snapshot_count_cache.get(&()) {
            Some(count) => Ok(count),
            None => {
                let count = database::retrieve_account_snapshot_count(&self.database).await?;
                self.account_snapshot_count_cache.insert((), count).await;
                Ok(count)
            }
        }
    }

    pub async fn retrieve_tank_snapshot_count(&self) -> crate::Result<i64> {
        match self.tank_snapshot_count_cache.get(&()) {
            Some(count) => Ok(count),
            None => {
                let count = database::retrieve_tank_snapshot_count(&self.database).await?;
                self.tank_snapshot_count_cache.insert((), count).await;
                Ok(count)
            }
        }
    }

    pub async fn retrieve_crawler_lag(&self) -> crate::Result<Duration> {
        match self.crawler_lag_cache.get(&()) {
            Some(lag) => Ok(lag),
            None => {
                let lag = Utc::now() - database::retrieve_oldest_crawled_at(&self.database).await?;
                self.crawler_lag_cache.insert((), lag).await;
                Ok(lag)
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
                    .get_account_info(&account_ids)
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
                        .get_account_info(&[account_id])
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
        let account_id = account_info.general.id;
        match self.account_tanks_cache.get(&account_id) {
            Some((last_battle_time, snapshots))
                if last_battle_time == account_info.general.last_battle_time =>
            {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots)
            }
            _ => {
                let snapshots = Arc::new(self.api.get_merged_tanks(account_id).await?);
                self.account_tanks_cache
                    .insert(
                        account_id,
                        (account_info.general.last_battle_time, snapshots.clone()),
                    )
                    .await;
                Ok(snapshots)
            }
        }
    }

    pub async fn get_vehicle(&self, tank_id: i32) -> crate::Result<Vehicle> {
        Ok(self
            .tankopedia
            .get(&tank_id)
            .cloned() // FIXME: avoid `cloned()`.
            .unwrap_or_else(|| Self::new_hardcoded_vehicle(tank_id)))
    }

    fn new_hardcoded_vehicle(tank_id: i32) -> Vehicle {
        Vehicle {
            tank_id,
            name: format!("#{}", tank_id),
            tier: 0,
            is_premium: false,
            type_: TankType::Light, // FIXME
            nation: Nation::Other,
        }
    }

    fn make_tracking_code(opts: &Opts) -> PreEscaped<String> {
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
        PreEscaped(extra_html_headers.join(""))
    }
}
