pub mod models;

use crate::api::wargaming::models::{
    AccountId, AccountInfos, Accounts, ApiResponse, TanksStatistics,
};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use surf::Url;

#[derive(Clone)]
pub struct WargamingApi {
    application_id: Arc<String>,
    client: surf::Client,
}

impl WargamingApi {
    pub fn new(application_id: &str) -> WargamingApi {
        Self {
            application_id: Arc::new(application_id.to_string()),
            client: surf::client()
                .with(surf::middleware::Logger::new())
                .with(crate::api::middleware::UserAgent),
        }
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    pub async fn search_accounts(&self, query: &str) -> crate::Result<Accounts> {
        log::debug!("search_accounts: {}", query);
        self.call(Url::parse_with_params(
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
    pub async fn get_account_info(&self, account_id: AccountId) -> crate::Result<AccountInfos> {
        log::debug!("get_account_info: {}", account_id);
        self.call(Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/account/info/",
            &[
                ("application_id", self.application_id.as_str()),
                ("account_id", account_id.to_string().as_str()),
            ],
        )?)
        .await
    }

    /// See <https://developers.wargaming.net/reference/all/wotb/tanks/stats/>.
    pub async fn get_tanks_stats(&self, account_id: AccountId) -> crate::Result<TanksStatistics> {
        log::debug!("get_tanks_stats: {}", account_id);
        self.call(Url::parse_with_params(
            "https://api.wotblitz.ru/wotb/tanks/stats/",
            &[
                ("application_id", self.application_id.as_str()),
                ("account_id", account_id.to_string().as_str()),
            ],
        )?)
        .await
    }

    async fn call<T: DeserializeOwned>(&self, uri: impl AsRef<str>) -> crate::Result<T> {
        self.client
            .get(uri)
            .await
            .map_err(surf::Error::into_inner)?
            .body_json::<ApiResponse<T>>()
            .await
            .map_err(surf::Error::into_inner)?
            .into()
    }
}
