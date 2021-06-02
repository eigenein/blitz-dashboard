use std::any::type_name;

use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use serde::Deserialize;
use tide::Request;

use crate::logging::log_anyhow;
use crate::wargaming::models::{Account, FullInfo};
use crate::web::components::SEARCH_QUERY_LENGTH;
use crate::web::State;
use std::sync::Arc;

pub type Percentage = f32;

/// User search query.
#[derive(Deserialize)]
pub struct IndexQueryString {
    #[serde(default = "Option::default")]
    search: Option<String>,
}

pub struct IndexViewModel {
    pub accounts: Option<Vec<Account>>,
}

pub struct PlayerViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub wins: Percentage,
    pub survival: Percentage,
    pub full_info: Arc<FullInfo>,
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
        let full_info = Arc::new(state.api.get_full_account_info(account_id).await?);
        {
            let database = state.database.clone();
            let full_info = full_info.clone();
            async_std::task::spawn(async move {
                log_anyhow(database.upsert_full_info(&full_info).await);
            });
        }
        Ok(Self {
            account_id,
            nickname: full_info.account_info.nickname.clone(),
            created_at: full_info.account_info.created_at,
            last_battle_time: full_info.account_info.last_battle_time,
            wins: 100.0 * (full_info.account_info.statistics.all.wins as f32)
                / (full_info.account_info.statistics.all.battles as f32),
            survival: 100.0 * (full_info.account_info.statistics.all.survived_battles as f32)
                / (full_info.account_info.statistics.all.battles as f32),
            full_info,
        })
    }
}
