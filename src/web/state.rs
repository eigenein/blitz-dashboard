use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use maud::PreEscaped;
use moka::future::{Cache, CacheBuilder};
use sqlx::PgPool;

use crate::database;
use crate::models::{AccountInfo, Tank};
use crate::opts::Opts;
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    pub api: WargamingApi,
    pub database: PgPool,
    pub tracking_code: PreEscaped<String>,

    account_tanks_cache: Cache<i32, (DateTime<Utc>, Arc<Vec<Tank>>)>,
    account_count_cache: Cache<(), i64>,
    account_snapshot_count_cache: Cache<(), i64>,
    tank_snapshot_count_cache: Cache<(), i64>,
    crawler_lag_cache: Cache<(), Duration>,
}

impl State {
    pub async fn new(api: WargamingApi, database: PgPool, opts: &Opts) -> crate::Result<Self> {
        const FIVE_MINUTES: StdDuration = StdDuration::from_secs(300);

        let state = State {
            api,
            database,
            tracking_code: Self::make_tracking_code(&opts),
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
