use crate::api::wargaming::models::{AccountId, Accounts, Statistics, TankStatistics};
use crate::web::components::SEARCH_QUERY_LENGTH;
use crate::web::State;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tide::Request;

pub type Percentage = f32;

/// User search query.
#[derive(Deserialize)]
pub struct IndexQueryString {
    #[serde(default = "Option::default")]
    search: Option<String>,
}

pub struct IndexViewModel {
    pub accounts: Option<Accounts>,
}

pub struct PlayerViewModel {
    pub account_id: AccountId,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub wins: Percentage,
    pub survival: Percentage,
    pub all_statistics: Statistics,
    pub tanks_stats: Vec<TankStatistics>,
}

impl IndexViewModel {
    pub async fn new(request: Request<State>) -> crate::Result<Self> {
        let query: IndexQueryString = request.query().map_err(surf::Error::into_inner)?;
        if let Some(query) = query.search {
            if SEARCH_QUERY_LENGTH.contains(&query.len()) {
                return Ok(IndexViewModel {
                    accounts: Some(request.state().api.search_accounts(&query).await?),
                });
            }
        }
        Ok(Self { accounts: None })
    }
}

impl PlayerViewModel {
    pub async fn new(request: Request<State>) -> crate::Result<Self> {
        let account_id: AccountId = request
            .param("account_id")
            .map_err(surf::Error::into_inner)?
            .parse()?;
        let state = request.state();
        let mut account_infos = state.api.get_account_info(account_id).await?;
        let (_, account_info) = account_infos
            .drain()
            .next()
            .ok_or_else(|| anyhow!("account not found"))?;
        let (_, tanks_stats) = state
            .api
            .get_tanks_stats(account_id)
            .await?
            .drain()
            .next()
            .unwrap();
        {
            let database = state.database.clone();
            let account_info = account_info.clone();
            let tanks_stats = tanks_stats.clone();
            async_std::task::spawn(async move {
                database.save_snapshots(account_info, tanks_stats).await
            });
        }
        let all_statistics = account_info.statistics.all.clone();
        Ok(Self {
            account_id,
            nickname: account_info.nickname,
            created_at: account_info.created_at,
            last_battle_time: account_info.last_battle_time,
            wins: 100.0 * (all_statistics.wins as f32) / (all_statistics.battles as f32),
            survival: 100.0 * (all_statistics.survived_battles as f32)
                / (all_statistics.battles as f32),
            all_statistics,
            tanks_stats,
        })
    }
}
