use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use tide::Request;

use crate::web::state::State;

pub struct PlayerViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub has_recently_played: bool,
    pub is_inactive: bool,
    pub n_battles: i32,
    pub n_tanks: usize,
}

impl PlayerViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<PlayerViewModel> {
        let account_id: i32 = Self::parse_account_id(&request)?;
        log::info!("Player: #{}", account_id);

        let state = request.state();
        let account = state.get_account_info(account_id).await?;
        let tanks = state.get_tanks(account_id).await?;
        let all = &account.statistics.all;

        Ok(Self {
            account_id: account.basic.id,
            nickname: account.nickname.clone(),
            created_at: account.created_at,
            last_battle_time: account.basic.last_battle_time,
            n_battles: all.battles,
            has_recently_played: account.basic.last_battle_time > (Utc::now() - Duration::hours(1)),
            is_inactive: account.basic.last_battle_time < (Utc::now() - Duration::days(365)),
            n_tanks: tanks.len(),
        })
    }

    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        Ok(request
            .param("account_id")
            .map_err(surf::Error::into_inner)
            .context("missing account ID")?
            .parse()
            .context("invalid account ID")?)
    }
}
