use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use serde::de::DeserializeOwned;
use surf::Url;

mod middleware;
pub mod models;

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
                .with(middleware::UserAgent)
                .with(middleware::Timeout(std::time::Duration::from_secs(10))),
        }
    }

    /// See: <https://developers.wargaming.net/reference/all/wotb/account/list/>.
    pub async fn search_accounts(&self, query: &str) -> crate::Result<Vec<models::Account>> {
        log::debug!("Searching: {}", query);
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
    pub async fn get_account_info(
        &self,
        account_id: i32,
    ) -> crate::Result<Option<models::AccountInfo>> {
        self.call_by_account("https://api.wotblitz.ru/wotb/account/info/", account_id)
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

    pub async fn get_full_info(&self, account_id: i32) -> crate::Result<models::FullInfo> {
        let account_info = self
            .get_account_info(account_id)
            .await?
            .ok_or_else(|| anyhow!("account ID not found"))?;
        let tanks_statistics = self.get_tanks_stats(account_id).await?;
        let _tanks_achievements = self.get_tanks_achievements(account_id).await?;
        Ok(models::FullInfo {
            account_info,
            tanks_statistics,
        })
    }

    /// Convenience method for endpoints that return data in the form of a map by account ID.
    async fn call_by_account<T: DeserializeOwned>(
        &self,
        url: &str,
        account_id: i32,
    ) -> crate::Result<Option<T>> {
        let account_id = account_id.to_string();
        log::debug!("{} #{}", url, account_id);
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
        self.client
            .get(url.as_str())
            .await
            .map_err(surf::Error::into_inner)?
            .body_json::<models::ApiResponse<T>>()
            .await
            .map_err(surf::Error::into_inner)?
            .into()
    }
}
