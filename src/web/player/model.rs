use std::any::type_name;

use chrono::{DateTime, Duration, Utc};
use tide::Request;

use crate::wargaming::models::TankStatistics;
use crate::web::state::State;

pub struct PlayerViewModel {
    pub account_id: i32,

    pub nickname: String,

    pub created_at: DateTime<Utc>,

    pub last_battle_time: DateTime<Utc>,

    pub has_recently_played: bool,

    pub is_inactive: bool,

    /// Win precentage.
    pub wins: f32,

    /// Survived battles percentage.
    pub survival: f32,

    /// Hits vs shots percentage.
    pub hits: f32,

    pub n_battles: i32,

    /// Tank name with the longest life time.
    pub longest_life_time_tank: Option<TankStatistics>,

    /// Tank name with the most battle count.
    pub most_played_tank: Option<TankStatistics>,
}

impl PlayerViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<PlayerViewModel> {
        let account_id: i32 = Self::parse_account_id(&request)?;
        log::info!("{} #{}…", type_name::<Self>(), account_id);
        let account_info = request
            .state()
            .get_aggregated_account_info(account_id)
            .await?;
        Ok(Self::from(&account_info))
    }

    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        Ok(request
            .param("account_id")
            .map_err(surf::Error::into_inner)?
            .parse()?)
    }

    fn from(info: &crate::wargaming::models::AggregatedAccountInfo) -> Self {
        let account = &info.account;
        let all = &account.statistics.all;
        let longest_life_time_tank = info
            .tanks
            .iter()
            .map(|(tank, _)| tank)
            .max_by_key(|tank| tank.battle_life_time)
            .cloned();
        let most_played_tank = info
            .tanks
            .iter()
            .map(|(tank, _)| tank)
            .max_by_key(|tank| tank.all.battles)
            .cloned();

        Self {
            account_id: account.id,
            nickname: account.nickname.clone(),
            created_at: account.created_at,
            last_battle_time: account.last_battle_time,
            wins: 100.0 * (all.wins as f32) / (all.battles as f32),
            survival: 100.0 * (all.survived_battles as f32) / (all.battles as f32),
            hits: 100.0 * (all.hits as f32) / (all.shots as f32),
            n_battles: all.battles,
            has_recently_played: account.last_battle_time > (Utc::now() - Duration::hours(1)),
            is_inactive: account.last_battle_time < (Utc::now() - Duration::days(365)),
            longest_life_time_tank,
            most_played_tank,
        }
    }
}
