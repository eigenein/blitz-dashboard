use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

pub type AccountId = i32;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Account {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: AccountId,
}

pub type Accounts = Vec<Account>;

#[derive(Deserialize, Debug, PartialEq)]
pub struct AccountInfo {
    #[serde(rename = "account_id")]
    pub id: AccountId,

    pub nickname: String,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_battle_time: DateTime<Utc>,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,

    pub statistics: AccountInfoStatistics,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct AccountInfoStatistics {
    pub all: AccountInfoStatisticsDetails,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct AccountInfoStatisticsDetails {
    pub battles: i32,
    pub wins: i32,
    pub survived_battles: i32,
    pub win_and_survived: i32,
    pub damage_dealt: i32,
    pub damage_received: i32,
}

pub type AccountInfos = HashMap<String, AccountInfo>;

/// Generic Wargaming.net API error.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ApiResponse<T> {
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
pub struct ApiError {
    message: String,

    #[serde(default)]
    code: Option<u16>,

    #[serde(default)]
    field: Option<String>,
}

impl<T> From<ApiResponse<T>> for anyhow::Result<T> {
    fn from(response: ApiResponse<T>) -> anyhow::Result<T> {
        match response {
            ApiResponse::Data { data } => Ok(data),
            ApiResponse::Error { error } => anyhow::Result::Err(anyhow!(
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
    fn test_parse_search_accounts_error() -> crate::Result {
        let response: ApiResponse<Accounts> = serde_json::from_str(
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

    #[test]
    fn test_get_account_info_ok() -> crate::Result {
        serde_json::from_str::<ApiResponse<AccountInfos>>(
            // language=json
            r#"{"status":"ok","meta":{"count":1},"data":{"5589968":{"statistics":{"clan":{"spotted":0,"max_frags_tank_id":0,"hits":0,"frags":0,"max_xp":0,"max_xp_tank_id":0,"wins":0,"losses":0,"capture_points":0,"battles":0,"damage_dealt":0,"damage_received":0,"max_frags":0,"shots":0,"frags8p":0,"xp":0,"win_and_survived":0,"survived_battles":0,"dropped_capture_points":0},"all":{"spotted":5154,"max_frags_tank_id":20817,"hits":48542,"frags":5259,"max_xp":1917,"max_xp_tank_id":54289,"wins":3425,"losses":2609,"capture_points":4571,"battles":6056,"damage_dealt":6009041,"damage_received":4524728,"max_frags":6,"shots":63538,"frags8p":1231,"xp":4008483,"win_and_survived":2524,"survived_battles":2635,"dropped_capture_points":4415},"frags":null},"account_id":5589968,"created_at":1415225091,"updated_at":1621792747,"private":null,"last_battle_time":1621802244,"nickname":"eigenein"}}}"#,
        )?;
        Ok(())
    }
}
