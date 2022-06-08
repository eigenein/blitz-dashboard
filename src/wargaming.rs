//! Wargaming.net API.

use std::collections::{BTreeMap, HashMap};
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context};
use itertools::Itertools;
pub use models::*;
use reqwest::header::HeaderValue;
use reqwest::{header, Url};
use serde::de::DeserializeOwned;
use tracing::{debug, instrument, warn};

use crate::helpers::backoff::Backoff;
use crate::helpers::tracing::{format_duration, format_elapsed};
use crate::prelude::*;
use crate::wargaming::response::Response;

pub mod cache;
pub mod models;
pub mod response;

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
    const USER_AGENT: &'static str =
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    pub fn new(application_id: &str, timeout: StdDuration) -> Result<WargamingApi> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, HeaderValue::from_static(Self::USER_AGENT));
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(header::ACCEPT_ENCODING, HeaderValue::from_static("br, deflate, gzip"));
        headers.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
        let this = Self {
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
            request_counter: Arc::new(AtomicU32::new(0)),
        };
        Ok(this)
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    #[instrument(skip_all, fields(query = query))]
    pub async fn search_accounts(&self, query: &str) -> Result<Vec<models::FoundAccount>> {
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
    #[instrument(skip_all, level = "debug", fields(n_accounts = account_ids.len()))]
    pub async fn get_account_info(
        &self,
        account_ids: &[i32],
    ) -> Result<HashMap<String, Option<AccountInfo>>> {
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
    #[instrument(skip_all, level = "debug", fields(account_id = account_id))]
    pub async fn get_tanks_stats(&self, account_id: i32) -> Result<Vec<TankStatistics>> {
        Ok(self
            .call_by_account("https://api.wotblitz.ru/wotb/tanks/stats/", account_id)
            .await
            .with_context(|| format!("failed to get tanks stats for #{}", account_id))?
            .unwrap_or_default())
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/achievements/>.
    #[instrument(skip_all, fields(account_id = account_id))]
    pub async fn get_tanks_achievements(&self, account_id: i32) -> Result<Vec<TankAchievements>> {
        Ok(self
            .call_by_account("https://api.wotblitz.ru/wotb/tanks/achievements/", account_id)
            .await
            .with_context(|| format!("failed to get tanks achievements for #{}", account_id))?
            .unwrap_or_default())
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/encyclopedia/vehicles/>.
    #[tracing::instrument(skip_all)]
    pub async fn get_tankopedia(&self) -> Result<Tankopedia> {
        info!("retrieving the tankopedia…");
        self.call::<Tankopedia>(Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/encyclopedia/vehicles/",
            &[("application_id", self.application_id.as_str())],
        )?)
        .await
        .context("failed to get the tankopedia")
    }

    /// Convenience method for endpoints that return data in the form of a map by account ID.
    #[instrument(skip_all, level = "debug", fields(account_id = account_id))]
    async fn call_by_account<T: DeserializeOwned>(
        &self,
        url: &str,
        account_id: i32,
    ) -> Result<Option<T>> {
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

    #[instrument(skip_all, level = "debug", fields(path = url.path()))]
    async fn call<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        let mut backoff = Backoff::new(100, 25600);
        loop {
            match self.call_once(url.clone()).await {
                Ok(response) => match response {
                    Response::Data { data } => {
                        trace!("ok");
                        return Ok(data);
                    }
                    Response::Error { error } => {
                        let message = error.message.as_str();
                        match message {
                            "REQUEST_LIMIT_EXCEEDED" | "SOURCE_NOT_AVAILABLE" => {
                                // ♻️ Retrying for these particular errors.
                                warn!(
                                    code = error.code,
                                    n_attempts = backoff.n_attempts(),
                                    message,
                                );
                            }
                            _ => {
                                // 🥅 This is an unexpected API error.
                                return Err(anyhow!("{}/{}", error.code, error.message));
                            }
                        }
                    }
                },
                Err(error) if error.is_timeout() => {
                    // ♻️ The HTTP request has timed out. No action needed, retrying…
                    warn!(path = url.path(), "request timeout");
                }
                Err(error) => {
                    // ♻️ The TCP/HTTP request has failed for a different reason. Keep retrying for a while.
                    warn!(path = url.path(), n_attempts = backoff.n_attempts(), "{:#}", error);
                    if backoff.n_attempts() >= 10 {
                        // 🥅 Don't know what to do.
                        return Err(error).context("all attempts have failed");
                    }
                }
            };
            let sleep_duration = backoff.next();
            debug!(
                sleep_duration = format_duration(sleep_duration).as_str(),
                nr_attempt = backoff.n_attempts(),
                "retrying…",
            );
            tokio::time::sleep(sleep_duration).await;
        }
    }

    #[tracing::instrument(skip_all, level = "trace", fields(path = url.path()))]
    async fn call_once<T: DeserializeOwned>(
        &self,
        url: Url,
    ) -> StdResult<Response<T>, reqwest::Error> {
        let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);
        trace!(request_id, path = url.path(), "sending the request…");

        let start_instant = Instant::now();
        let result = self.client.get(url).send().await;

        let response = match result {
            Ok(result) => result,
            Err(error) => {
                return Err(error);
            }
        };

        trace!(request_id, status = response.status().as_u16());
        let response = response.error_for_status()?;

        trace!(request_id, type_name = std::any::type_name::<T>());
        let result = response.json::<Response<T>>().await;

        trace!(request_id, elapsed = format_elapsed(start_instant).as_str(), "done");
        result
    }
}
