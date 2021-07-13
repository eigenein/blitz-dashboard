use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::{anyhow, Context};
use clap::{crate_name, crate_version};
use itertools::{merge_join_by, EitherOrBoth, Itertools};
use moka::future::{Cache, CacheBuilder};
use reqwest::header;
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::models;

#[derive(Clone)]
pub struct WargamingApi {
    application_id: Arc<String>,
    client: reqwest::Client,
    request_counter: Arc<AtomicU32>,
}

/// Generic Wargaming.net API response.
#[derive(Deserialize)]
struct Response<T> {
    data: T,
}

impl WargamingApi {
    pub fn new(application_id: &str) -> crate::Result<WargamingApi> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(concat!(crate_name!(), "/", crate_version!())),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT_ENCODING,
            header::HeaderValue::from_static("br, deflate, gzip"),
        );
        Ok(Self {
            application_id: Arc::new(application_id.to_string()),
            client: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .https_only(true)
                .timeout(StdDuration::from_secs(3))
                .build()?,
            request_counter: Arc::new(AtomicU32::new(1)),
        })
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
        .with_context(|| format!("failed to search for accounts: `{}`", query))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/account/info/>.
    pub async fn get_account_info(
        &self,
        account_ids: &[i32],
    ) -> crate::Result<HashMap<String, Option<models::AccountInfo>>> {
        if account_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let account_id = account_ids
            .iter()
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
        .with_context(|| format!("failed to get account infos: `{}`", account_id))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/stats/>.
    pub async fn get_tanks_stats(
        &self,
        account_id: i32,
    ) -> crate::Result<Vec<models::TankStatistics>> {
        Ok(self
            .call_by_account("https://api.wotblitz.ru/wotb/tanks/stats/", account_id)
            .await
            .with_context(|| format!("failed to get tanks stats for account #{}", account_id))?
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
            .await
            .with_context(|| {
                format!(
                    "failed to get tanks achievements for account #{}",
                    account_id,
                )
            })?
            .unwrap_or_else(Vec::new))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/encyclopedia/vehicles/>.
    pub async fn get_tankopedia(&self) -> crate::Result<HashMap<i32, models::Vehicle>> {
        log::info!("Retrieving the tankopedia…");
        Ok(self
            .call::<HashMap<String, models::Vehicle>>(&Url::parse_with_params(
                "https://api.wotblitz.ru/wotb/encyclopedia/vehicles/",
                &[("application_id", self.application_id.as_str())],
            )?)
            .await
            .context("failed to get the tankopedia")?
            .into_iter()
            .map(|(tank_id, vehicle)| {
                tank_id
                    .parse::<i32>()
                    .map(|tank_id| (tank_id, vehicle))
                    .map_err(|error| anyhow!(error))
            })
            .collect::<crate::Result<HashMap<i32, models::Vehicle>>>()
            .context("failed to parse the tankopedia")?)
    }

    pub async fn get_merged_tanks(&self, account_id: i32) -> crate::Result<Vec<models::Tank>> {
        let mut statistics = self.get_tanks_stats(account_id).await?;
        let mut achievements = self.get_tanks_achievements(account_id).await?;

        statistics.sort_unstable_by_key(|snapshot| snapshot.tank_id);
        achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

        let tanks: Vec<models::Tank> = merge_join_by(statistics, achievements, |left, right| {
            left.tank_id.cmp(&right.tank_id)
        })
        .filter_map(|item| match item {
            EitherOrBoth::Both(statistics, _achievements) => Some(models::Tank {
                account_id,
                tank_id: statistics.tank_id,
                all_statistics: statistics.all,
                last_battle_time: statistics.last_battle_time,
                battle_life_time: statistics.battle_life_time,
            }),
            _ => None,
        })
        .collect();
        Ok(tanks)
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
        let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);
        log::debug!("Sending #{} {}", request_id, url);
        let start_instant = Instant::now();
        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .context("failed to send the Wargaming.net API request")?;
        log::debug!(
            "Done #{} [{}] {:?}",
            request_id,
            response.status(),
            Instant::now() - start_instant,
        );
        Ok(response
            .error_for_status()
            .context("Wargaming.net API request has failed")?
            .json::<Response<T>>()
            .await
            .context("failed to parse JSON")?
            .data)
    }
}

const DAY: StdDuration = StdDuration::from_secs(86400);
const MINUTE: StdDuration = StdDuration::from_secs(60);
const FIVE_MINUTES: StdDuration = StdDuration::from_secs(300);

pub struct AccountSearchCache {
    api: WargamingApi,
    id_cache: Cache<String, Arc<Vec<i32>>>,
    info_cache: Cache<String, Arc<Vec<models::AccountInfo>>>,
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
    pub async fn get(&self, query: &String) -> crate::Result<Arc<Vec<models::AccountInfo>>> {
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

                let account_infos: Vec<models::AccountInfo> = self
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

/// Cached account info.
pub struct AccountInfoCache {
    api: WargamingApi,
    cache: Cache<i32, Arc<models::AccountInfo>>,
}

impl AccountInfoCache {
    pub fn new(api: WargamingApi) -> Self {
        Self {
            api,
            cache: CacheBuilder::new(1_000).time_to_live(MINUTE).build(),
        }
    }

    pub async fn get(&self, account_id: i32) -> crate::Result<Arc<models::AccountInfo>> {
        match self.cache.get(&account_id) {
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
                self.cache.insert(account_id, account_info.clone()).await;
                Ok(account_info)
            }
        }
    }
}
