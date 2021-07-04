use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context};
use itertools::{merge_join_by, EitherOrBoth, Itertools};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use surf::Url;

use crate::models;

mod middleware;

#[derive(Clone)]
pub struct WargamingApi {
    application_id: Arc<String>,
    client: surf::Client,
}

/// Generic Wargaming.net API response.
#[derive(Deserialize)]
struct Response<T> {
    data: T,
}

impl WargamingApi {
    pub fn new(application_id: &str) -> WargamingApi {
        Self {
            application_id: Arc::new(application_id.to_string()),
            client: surf::client()
                .with(middleware::UserAgent)
                .with(middleware::TimeoutAndRetry {
                    timeout: StdDuration::from_millis(1000),
                    n_retries: 3,
                })
                .with(middleware::Logger),
        }
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    pub async fn search_accounts(&self, query: &str) -> crate::Result<Vec<models::Account>> {
        self.call(&Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/account/list/",
            &[
                ("application_id", self.application_id.as_str()),
                ("limit", "20"),
                ("search", query),
            ],
        )?)
        .await
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/account/info/>.
    pub async fn get_account_info<A: IntoIterator<Item = I>, I: ToString>(
        &self,
        account_ids: A,
    ) -> crate::Result<HashMap<String, Option<models::AccountInfo>>> {
        let account_id = account_ids
            .into_iter()
            .map(|account_id| account_id.to_string())
            .join(",");
        self.call(&Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/account/info/",
            &[
                ("application_id", self.application_id.as_str()),
                ("account_id", account_id.as_str()),
            ],
        )?)
        .await
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/stats/>.
    pub async fn get_tanks_stats(
        &self,
        account_id: i32,
    ) -> crate::Result<Vec<models::TankStatistics>> {
        Ok(self
            .call_by_account("https://api.wotblitz.ru/wotb/tanks/stats/", account_id)
            .await?
            .unwrap_or_else(Vec::new))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/achievements/>.
    pub async fn get_tanks_achievements(
        &self,
        account_id: i32,
    ) -> crate::Result<Vec<models::TankAchievements>> {
        Ok(self
            .call_by_account(
                "https://api.wotblitz.ru/wotb/tanks/achievements/",
                account_id,
            )
            .await?
            .unwrap_or_else(Vec::new))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/encyclopedia/vehicles/>.
    pub async fn get_tankopedia(&self) -> crate::Result<HashMap<i32, models::Vehicle>> {
        Ok(self
            .call::<HashMap<String, models::Vehicle>>(&Url::parse_with_params(
                "https://api.wotblitz.ru/wotb/encyclopedia/vehicles/",
                &[("application_id", self.application_id.as_str())],
            )?)
            .await?
            .into_iter()
            .map(|(tank_id, vehicle)| {
                tank_id
                    .parse::<i32>()
                    .map(|tank_id| (tank_id, vehicle))
                    .map_err(|error| anyhow!(error))
            })
            .collect::<crate::Result<HashMap<i32, models::Vehicle>>>()?)
    }

    pub async fn get_merged_tanks(
        &self,
        account_id: i32,
    ) -> crate::Result<Vec<models::TankSnapshot>> {
        let mut statistics = self.get_tanks_stats(account_id).await?;
        let mut achievements = self.get_tanks_achievements(account_id).await?;

        statistics.sort_unstable_by_key(|snapshot| snapshot.tank_id);
        achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

        Ok(merge_join_by(statistics, achievements, |left, right| {
            left.tank_id.cmp(&right.tank_id)
        })
        .filter_map(|item| match item {
            EitherOrBoth::Both(statistics, _achievements) => Some(models::TankSnapshot {
                account_id,
                tank_id: statistics.tank_id,
                all_statistics: statistics.all,
                last_battle_time: statistics.last_battle_time,
                battle_life_time: statistics.battle_life_time,
            }),
            _ => None,
        })
        .collect::<Vec<models::TankSnapshot>>())
    }

    /// Convenience method for endpoints that return data in the form of a map by account ID.
    async fn call_by_account<T: DeserializeOwned>(
        &self,
        url: &str,
        account_id: i32,
    ) -> crate::Result<Option<T>> {
        let account_id = account_id.to_string();
        Ok(self
            .call::<HashMap<String, Option<T>>>(&Url::parse_with_params(
                url,
                &[
                    ("application_id", self.application_id.as_str()),
                    ("account_id", account_id.as_str()),
                ],
            )?)
            .await?
            .remove(&account_id)
            .flatten())
    }

    async fn call<T: DeserializeOwned>(&self, url: &Url) -> crate::Result<T> {
        Ok(self
            .client
            .get(url.as_str())
            .await
            .map_err(surf::Error::into_inner)
            .context("request has failed")?
            .body_json::<Response<T>>()
            .await
            .map_err(surf::Error::into_inner)
            .context("could not parse JSON")?
            .data)
    }
}
