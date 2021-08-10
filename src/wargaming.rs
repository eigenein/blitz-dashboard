use std::collections::{BTreeMap, HashMap};
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::{anyhow, Context};
use itertools::{merge_join_by, EitherOrBoth, Itertools};
use reqwest::header;
use reqwest::Url;
use sentry::{capture_message, Level};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::backoff::Backoff;
use crate::metrics::RpsCounter;
use crate::models;
use crate::wargaming::response::Response;

pub mod cache;
pub mod response;

#[derive(Clone)]
pub struct WargamingApi {
    application_id: Arc<String>,
    client: reqwest::Client,
    request_counter: Arc<AtomicU32>,
    rps_counter: Arc<RwLock<RpsCounter>>,
}

/// Represents the bundled `tankopedia.json` file.
/// Note, that I'm using [`BTreeMap`] to keep the keys sorted in the output file for better diffs.
pub type Tankopedia = BTreeMap<String, serde_json::Value>;

impl WargamingApi {
    pub fn new(application_id: &str, timeout: StdDuration) -> crate::Result<WargamingApi> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION"),
            )),
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
                .timeout(timeout)
                .brotli(true)
                .gzip(true)
                .deflate(true)
                .tcp_nodelay(true)
                .build()?,
            request_counter: Arc::new(AtomicU32::new(1)),
            rps_counter: Arc::new(RwLock::new(RpsCounter::new("API", 50))),
        })
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    pub async fn search_accounts(&self, query: &str) -> crate::Result<Vec<models::FoundAccount>> {
        self.call(Url::parse_with_params(
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
        self.call(Url::parse_with_params(
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
            .with_context(|| format!("failed to get tanks stats for #{}", account_id))?
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
            .with_context(|| format!("failed to get tanks achievements for #{}", account_id))?
            .unwrap_or_else(Vec::new))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/encyclopedia/vehicles/>.
    pub async fn get_tankopedia(&self) -> crate::Result<Tankopedia> {
        log::info!("Retrieving the tankopedia…");
        self.call::<Tankopedia>(Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/encyclopedia/vehicles/",
            &[("application_id", self.application_id.as_str())],
        )?)
        .await
        .context("failed to get the tankopedia")
    }

    pub async fn get_merged_tanks(&self, account_id: i32) -> crate::Result<Vec<models::Tank>> {
        let mut statistics = self.get_tanks_stats(account_id).await?;
        let mut achievements = self.get_tanks_achievements(account_id).await?;

        statistics.sort_unstable_by_key(|snapshot| snapshot.base.tank_id);
        achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

        let tanks: Vec<models::Tank> = merge_join_by(statistics, achievements, |left, right| {
            left.base.tank_id.cmp(&right.tank_id)
        })
        .filter_map(|item| match item {
            EitherOrBoth::Both(statistics, _achievements) => Some(models::Tank {
                account_id,
                tank_id: statistics.base.tank_id,
                all_statistics: statistics.all,
                last_battle_time: statistics.base.last_battle_time,
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
        let mut map: HashMap<String, Option<T>> = self
            .call(Url::parse_with_params(
                url,
                &[
                    ("application_id", self.application_id.as_str()),
                    ("account_id", account_id.as_str()),
                ],
            )?)
            .await?;
        Ok(map.remove(&account_id).flatten())
    }

    async fn call<T: DeserializeOwned>(&self, url: Url) -> crate::Result<T> {
        let mut backoff = Backoff::new(100, 25600);
        loop {
            match self.call_once(url.clone()).await {
                Ok(response) => {
                    match response {
                        Response::Data { data } => {
                            // 🎉 The request has simply succeeded.
                            return Ok(data);
                        }
                        Response::Error { error } => match error.message.as_str() {
                            "REQUEST_LIMIT_EXCEEDED" => {
                                // ♻️ The HTTP request has succeeded, but we've reached the RPS limit.
                                log::warn!("Exceeded the request limit.");
                                capture_message("Exceeded the API RPS limit", Level::Warning);
                            }
                            "SOURCE_NOT_AVAILABLE" => {
                                // ♻️ The HTTP request has succeeded, but the API has an issue.
                                log::warn!("API source is unavailable.");
                                capture_message("API source is unavailable", Level::Info);
                            }
                            _ => {
                                // 🥅 The HTTP request has succeeded, but the API has returned an unexpected error.
                                return Err(anyhow!("{}/{}", error.code, error.message));
                            }
                        },
                    }
                }
                Err(error) if error.is_timeout() => {
                    // ♻️ The HTTP request has timed out. Retrying…
                    log::warn!("Wargaming.net API has timed out.");
                    capture_message("Wargaming.net API has timed out", Level::Info);
                }
                Err(error) => {
                    // ♻️ The TCP/HTTP request has failed for a different reason. Keep retrying for a while.
                    if backoff.attempts() >= 10 {
                        // 🥅 Don't know what to do.
                        return Err(error).context("failed to call the Wargaming.net API");
                    }
                }
            };
            let sleep_duration = backoff.next();
            log::warn!("Retrying in {:?}…", sleep_duration);
            tokio::time::sleep(sleep_duration).await;
        }
    }

    async fn call_once<T: DeserializeOwned>(
        &self,
        url: Url,
    ) -> StdResult<Response<T>, reqwest::Error> {
        let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);
        log::debug!("Get #{} {}", request_id, url);

        let start_instant = Instant::now();
        let result = self.client.get(url).send().await;
        let elapsed = Instant::now() - start_instant;
        log::debug!("Done #{} in {:?}", request_id, elapsed);

        if let Err(error) = &result {
            if error.is_timeout() {
                log::warn!("#{} has timed out.", request_id);
            } else {
                log::warn!("#{} has failed.", request_id);
            }
        }
        self.rps_counter.write().await.increment();
        result?.error_for_status()?.json::<Response<T>>().await
    }
}
