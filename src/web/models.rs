use crate::api::wargaming::models::{AccountId, Statistics, TankStatistics};
use crate::web::State;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use tide::Request;

pub type Percentage = f32;

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
        Ok(PlayerViewModel {
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
