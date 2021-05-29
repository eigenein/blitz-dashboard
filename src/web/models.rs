use std::any::type_name;

use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use serde::Deserialize;
use tide::Request;

use crate::logging::log_anyhow;
use crate::wargaming::models::{Accounts, Statistics, TankStatistics};
use crate::web::components::SEARCH_QUERY_LENGTH;
use crate::web::State;

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
    pub account_id: i32,
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
        log::debug!("{} {:?}…", type_name::<Self>(), query.search);
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
        let account_id: i32 = request
            .param("account_id")
            .map_err(surf::Error::into_inner)?
            .parse()?;
        log::info!("{} #{}…", type_name::<Self>(), account_id);
        let state = request.state();
        let (account_info, tanks_stats) = state.api.get_full_account_info(account_id).await?;
        {
            let database = state.database.clone();
            let account_info = account_info.clone();
            let tanks_stats = tanks_stats.clone();
            async_std::task::spawn(async move {
                log_anyhow(
                    database
                        .upsert_account_info(&account_info, &tanks_stats)
                        .await,
                );
            });
        }
        let all = account_info.statistics.all;
        Ok(Self {
            account_id,
            nickname: account_info.nickname,
            created_at: account_info.created_at,
            last_battle_time: account_info.last_battle_time,
            wins: 100.0 * (all.wins as f32) / (all.battles as f32),
            survival: 100.0 * (all.survived_battles as f32) / (all.battles as f32),
            all_statistics: all,
            tanks_stats,
        })
    }
}
