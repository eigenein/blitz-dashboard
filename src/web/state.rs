use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use maud::PreEscaped;
use moka::future::{Cache, CacheBuilder};
use sqlx::PgPool;

use crate::models::{AccountInfo, Tank};
use crate::opts::Opts;
use crate::wargaming::WargamingApi;

/// Web application global state.
#[derive(Clone)]
pub struct State {
    // TODO: move to Rocket's `.manage()`.
    api: WargamingApi,
    pub database: PgPool,

    // TODO: move into a separate struct.
    pub tracking_code: PreEscaped<String>,

    // TODO: forgot to move into a separate struct.
    #[allow(clippy::type_complexity)]
    account_tanks_cache: Cache<i32, (DateTime<Utc>, Arc<HashMap<i32, Tank>>)>,
}

impl State {
    pub async fn new(api: WargamingApi, database: PgPool, opts: &Opts) -> crate::Result<Self> {
        let state = State {
            api,
            database,
            tracking_code: Self::make_tracking_code(&opts),
            account_tanks_cache: CacheBuilder::new(1_000).build(),
        };
        Ok(state)
    }

    pub async fn retrieve_tanks(
        &self,
        account_info: &AccountInfo,
    ) -> crate::Result<Arc<HashMap<i32, Tank>>> {
        let account_id = account_info.general.id;
        match self.account_tanks_cache.get(&account_id) {
            Some((last_battle_time, snapshots))
                if last_battle_time == account_info.general.last_battle_time =>
            {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots)
            }
            _ => {
                let snapshots: Arc<HashMap<i32, Tank>> = Arc::new(
                    self.api
                        .get_merged_tanks(account_id)
                        .await?
                        .into_iter()
                        .map(|tank| (tank.tank_id, tank))
                        .collect(),
                );
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
