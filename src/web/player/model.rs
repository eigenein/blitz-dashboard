use std::any::type_name;

use chrono::{DateTime, Utc};
use tide::Request;

use crate::web::state::State;

pub type Percentage = f32;

pub struct PlayerViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub wins: Percentage,
    pub survival: Percentage,
    pub n_battles: i32,
}

impl PlayerViewModel {
    pub async fn new(request: Request<State>) -> crate::Result<Self> {
        let account_id: i32 = Self::parse_account_id(&request)?;
        log::info!("{} #{}…", type_name::<Self>(), account_id);
        let account_info = request
            .state()
            .get_aggregated_account_info(account_id)
            .await?;
        Ok(Self::from(&account_info.account_info))
    }

    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        Ok(request
            .param("account_id")
            .map_err(surf::Error::into_inner)?
            .parse()?)
    }

    fn from(account_info: &crate::wargaming::models::AccountInfo) -> Self {
        let all = &account_info.statistics.all;
        Self {
            account_id: account_info.id,
            nickname: account_info.nickname.clone(),
            created_at: account_info.created_at,
            last_battle_time: account_info.last_battle_time,
            wins: 100.0 * (all.wins as f32) / (all.battles as f32),
            survival: 100.0 * (all.survived_battles as f32) / (all.battles as f32),
            n_battles: all.battles,
        }
    }
}
