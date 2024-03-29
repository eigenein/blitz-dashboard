//! Wargaming.net API.

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Jitter, Quota, RateLimiter};
use itertools::Itertools;
pub use models::*;
use reqwest::header::HeaderValue;
use reqwest::{header, Url};
use serde::de::DeserializeOwned;
use tokio::time::sleep;
use tracing::{debug, instrument, warn};

use crate::helpers::tracing::format_elapsed;
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
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

/// Represents the bundled `tankopedia.json` file.
/// Note, that I'm using [`BTreeMap`] to keep the keys sorted in the output file for better diffs.
pub type Tankopedia = BTreeMap<String, Vehicle>;

impl WargamingApi {
    const USER_AGENT: &'static str =
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    pub fn new(
        application_id: &str,
        timeout: time::Duration,
        max_rps: NonZeroU32,
    ) -> Result<WargamingApi> {
        info!(max_rps);

        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, HeaderValue::from_static(Self::USER_AGENT));
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(header::ACCEPT_ENCODING, HeaderValue::from_static("br, deflate, gzip"));
        headers.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));

        let rate_limiter = RateLimiter::direct(Quota::per_second(max_rps));

        let this = Self {
            application_id: Arc::new(application_id.to_string()),
            client: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .https_only(true)
                .timeout(timeout)
                .connect_timeout(timeout)
                .brotli(true)
                .gzip(true)
                .deflate(true)
                .tcp_nodelay(true)
                .pool_max_idle_per_host(0) // https://github.com/hyperium/hyper/issues/2312
                .build()?,
            request_counter: Arc::new(AtomicU32::new(0)),
            rate_limiter: Arc::new(rate_limiter),
        };
        Ok(this)
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    #[instrument(skip_all, fields(realm = ?realm, query = query))]
    pub async fn search_accounts(&self, realm: Realm, query: &str) -> Result<Vec<FoundAccount>> {
        let url = match realm {
            Realm::Asia => "https://api.wotblitz.asia/wotb/account/list/",
            Realm::Europe => "https://api.wotblitz.eu/wotb/account/list/",
            Realm::Russia => "https://api.wotblitz.ru/wotb/account/list/",
            Realm::NorthAmerica => "https://api.wotblitz.com/wotb/account/list/",
        };
        self.call(Url::parse_with_params(
            url,
            &[
                ("application_id", self.application_id.as_str()),
                ("limit", "20"),
                ("search", query),
            ],
        )?)
        .await
        .with_context(|| format!("failed to search for accounts: `{query}`"))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/account/info/>.
    #[instrument(skip_all, level = "debug", fields(realm = ?realm, n_accounts = account_ids.len()))]
    pub async fn get_account_info(
        &self,
        realm: Realm,
        account_ids: &[AccountId],
    ) -> Result<HashMap<String, Option<AccountInfo>>> {
        if account_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let account_id = account_ids.iter().map(ToString::to_string).join(",");
        let url = match realm {
            Realm::Asia => "https://api.wotblitz.asia/wotb/account/info/",
            Realm::Europe => "https://api.wotblitz.eu/wotb/account/info/",
            Realm::Russia => "https://api.wotblitz.ru/wotb/account/info/",
            Realm::NorthAmerica => "https://api.wotblitz.com/wotb/account/info/",
        };
        self.call(Url::parse_with_params(
            url,
            &[
                ("application_id", self.application_id.as_str()),
                ("account_id", account_id.as_str()),
                ("extra", "statistics.rating"),
            ],
        )?)
        .await
        .with_context(|| format!("failed to get account infos: `{account_id}`"))
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/stats/>.
    #[instrument(skip_all, level = "debug", fields(realm = ?realm, account_id = account_id))]
    pub async fn get_tanks_stats(
        &self,
        realm: Realm,
        account_id: AccountId,
    ) -> Result<Vec<TankStats>> {
        let url = match realm {
            Realm::Asia => "https://api.wotblitz.asia/wotb/tanks/stats/",
            Realm::Europe => "https://api.wotblitz.eu/wotb/tanks/stats/",
            Realm::Russia => "https://api.wotblitz.ru/wotb/tanks/stats/",
            Realm::NorthAmerica => "https://api.wotblitz.com/wotb/tanks/stats/",
        };
        Ok(self
            .call_by_account(url, account_id)
            .await
            .with_context(|| format!("failed to get tanks stats for #{account_id}"))?
            .unwrap_or_default())
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/achievements/>.
    #[instrument(skip_all, fields(account_id = account_id))]
    pub async fn get_tanks_achievements(
        &self,
        realm: Realm,
        account_id: AccountId,
    ) -> Result<Vec<TankAchievements>> {
        let url = match realm {
            Realm::Asia => "https://api.wotblitz.asia/wotb/tanks/achievements/",
            Realm::Europe => "https://api.wotblitz.eu/wotb/tanks/achievements/",
            Realm::Russia => "https://api.wotblitz.ru/wotb/tanks/achievements/",
            Realm::NorthAmerica => "https://api.wotblitz.com/wotb/tanks/achievements/",
        };
        Ok(self
            .call_by_account(url, account_id)
            .await
            .with_context(|| format!("failed to get tanks achievements for #{account_id}"))?
            .unwrap_or_default())
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/encyclopedia/vehicles/>.
    #[tracing::instrument(skip_all)]
    pub async fn get_tankopedia(&self) -> Result<Tankopedia> {
        info!("retrieving the tankopedia…");
        self.call::<Tankopedia>(Url::parse_with_params(
            "https://api.wotblitz.eu/wotb/encyclopedia/vehicles/",
            &[
                ("application_id", self.application_id.as_str()),
                ("language", "en"),
            ],
        )?)
        .await
        .context("failed to get the tankopedia")
    }

    /// Convenience method for endpoints that return data in the form of a map by account ID.
    #[instrument(skip_all, level = "debug", fields(account_id = account_id))]
    async fn call_by_account<T: DeserializeOwned>(
        &self,
        url: &str,
        account_id: AccountId,
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

    #[instrument(skip_all, fields(path = url.path()), err)]
    async fn call<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        for nr_attempt in 1..=10 {
            match self.call_once(url.clone()).await {
                Ok(response) => match response {
                    Response::Data { data } => {
                        trace!(nr_attempt, "ok");
                        return Ok(data);
                    }
                    Response::Error { error } => {
                        let message = error.message.as_str();
                        match message {
                            "REQUEST_LIMIT_EXCEEDED" => {
                                warn!(error.code, nr_attempt, "request limit exceeded");
                            }
                            "SOURCE_NOT_AVAILABLE" => {
                                warn!(error.code, nr_attempt, "source not available");
                                sleep(time::Duration::from_secs(1)).await;
                            }
                            _ => {
                                bail!("#{} {}/{}", nr_attempt, error.code, message);
                            }
                        }
                    }
                },
                Err(error) => {
                    warn!(path = url.path(), nr_attempt, "{:#}", error);
                }
            };
            debug!(nr_attempt, "retrying…");
        }
        bail!("all attempts have failed")
    }

    #[tracing::instrument(skip_all, fields(path = url.path()))]
    async fn call_once<T: DeserializeOwned>(&self, url: Url) -> Result<Response<T>> {
        self.rate_limiter
            .until_ready_with_jitter(Jitter::up_to(time::Duration::from_millis(100)))
            .await;

        let nr_request = self.request_counter.fetch_add(1, Ordering::Relaxed);
        trace!(nr_request, path = url.path(), "sending the request…");

        let start_instant = Instant::now();
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("failed to send the request")?;

        trace!(nr_request, status = ?response.status());
        let result = response
            .error_for_status()
            .context("HTTP error")?
            .json::<Response<T>>()
            .await
            .context("failed to deserialize the response");

        trace!(nr_request, elapsed = format_elapsed(start_instant).as_str(), "done");
        result
    }
}
