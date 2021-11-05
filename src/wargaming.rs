//! Wargaming.net API.

use std::collections::{BTreeMap, HashMap};
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context};
use itertools::Itertools;
use reqwest::header;
use reqwest::Url;
use sentry::{capture_message, Level};
use serde::de::DeserializeOwned;

use crate::backoff::Backoff;
use crate::helpers::format_duration;
use crate::models;
use crate::wargaming::response::Response;
use crate::StdDuration;

pub mod cache;
pub mod response;
pub mod tank_id;

#[derive(Clone)]
pub struct WargamingApi {
    pub request_counter: Arc<AtomicU32>,

    application_id: Arc<String>,
    client: reqwest::Client,
}

/// Represents the bundled `tankopedia.json` file.
/// Note, that I'm using [`BTreeMap`] to keep the keys sorted in the output file for better diffs.
pub type Tankopedia = BTreeMap<String, serde_json::Value>;

impl WargamingApi {
    pub fn new(application_id: &str) -> crate::Result<WargamingApi> {
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
        let this = Self {
            application_id: Arc::new(application_id.to_string()),
            client: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .https_only(true)
                .timeout(StdDuration::from_secs(5))
                .brotli(true)
                .gzip(true)
                .deflate(true)
                .tcp_nodelay(true)
                .build()?,
            request_counter: Arc::new(AtomicU32::new(0)),
        };
        Ok(this)
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
        let account_id = account_ids.iter().map(ToString::to_string).join(",");
        self.call(Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/account/info/",
            &[
                ("application_id", self.application_id.as_str()),
                ("account_id", account_id.as_str()),
                ("extra", "statistics.rating"),
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

    #[tracing::instrument(level = "debug", skip(self))]
    async fn call<T: DeserializeOwned>(&self, url: Url) -> crate::Result<T> {
        let mut backoff = Backoff::new(100, 25600);
        loop {
            match self.call_once(url.clone()).await {
                Ok(response) => match response {
                    Response::Data { data } => {
                        return Ok(data);
                    }
                    Response::Error { error } => {
                        let message = error.message.as_str();
                        match message {
                            "REQUEST_LIMIT_EXCEEDED" | "SOURCE_NOT_AVAILABLE" => {
                                tracing::warn!(message = message);
                                capture_message(message, Level::Warning);
                            }
                            _ => {
                                return Err(anyhow!("{}/{}", error.code, error.message));
                            }
                        }
                    }
                },
                Err(error) if error.is_timeout() => {
                    // ♻️ The HTTP request has timed out. No action needed, retrying…
                }
                Err(error) => {
                    // ♻️ The TCP/HTTP request has failed for a different reason. Keep retrying for a while.
                    if backoff.n_attempts() >= 10 {
                        // 🥅 Don't know what to do.
                        return Err(error).context("failed to call the Wargaming.net API");
                    }
                }
            };
            let sleep_duration = backoff.next();
            tracing::warn!(
                sleep_duration = format_duration(sleep_duration).as_str(),
                n_attempts = backoff.n_attempts(),
                "retrying…",
            );
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
                log::warn!("#{} has failed: {:#}.", request_id, error);
            }
        }
        result?.error_for_status()?.json::<Response<T>>().await
    }
}
