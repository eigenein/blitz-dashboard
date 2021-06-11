use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use itertools::{merge_join_by, EitherOrBoth};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use surf::Url;

use crate::models;
use crate::models::Tank;

mod middleware;

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

    pub async fn get_merged_tanks(&self, account_id: i32) -> crate::Result<Vec<Tank>> {
        let mut statistics = self.get_tanks_stats(account_id).await?;
        let mut achievements = self.get_tanks_achievements(account_id).await?;

        statistics.sort_by_key(|tank| tank.tank_id);
        achievements.sort_by_key(|tank| tank.tank_id);

        Ok(merge_join_by(statistics, achievements, |left, right| {
            left.tank_id.cmp(&right.tank_id)
        })
        .filter_map(|item| match item {
            EitherOrBoth::Both(statistics, achievements) => Some(Tank {
                account_id,
                tank_id: statistics.tank_id,
                all_statistics: statistics.all,
                last_battle_time: statistics.last_battle_time,
                battle_life_time: statistics.battle_life_time,
                max_series: achievements.max_series,
                achievements: achievements.achievements,
            }),
            _ => None,
        })
        .collect::<Vec<Tank>>())
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
            .body_json::<ApiResponse<T>>()
            .await
            .map_err(surf::Error::into_inner)?
            .into()
    }
}

/// Generic Wargaming.net API error.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum ApiResponse<T> {
    Data {
        data: T,
    },

    /// See: <https://developers.wargaming.net/documentation/guide/getting-started/#common-errors>
    Error {
        error: ApiError,
    },
}

/// Wargaming.net API error.
#[derive(Deserialize, Debug, PartialEq)]
struct ApiError {
    message: String,

    #[serde(default)]
    code: Option<u16>,

    #[serde(default)]
    field: Option<String>,
}

impl<T> From<ApiResponse<T>> for crate::Result<T> {
    fn from(response: ApiResponse<T>) -> crate::Result<T> {
        match response {
            ApiResponse::Data { data } => Ok(data),
            ApiResponse::Error { error } => crate::Result::Err(anyhow!(
                r#"[{}] "{}" in "{}""#,
                error.code.unwrap_or_default(),
                error.message,
                error.field.unwrap_or_default(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accounts_error() -> crate::Result {
        let response: ApiResponse<()> = serde_json::from_str(
            // language=json
            r#"{"status":"error","error":{"field":"search","message":"INVALID_SEARCH","code":407,"value":"1 2"}}"#,
        )?;
        assert_eq!(
            response,
            ApiResponse::Error {
                error: ApiError {
                    message: "INVALID_SEARCH".to_string(),
                    code: Some(407),
                    field: Some("search".to_string()),
                }
            }
        );
        Ok(())
    }
}
