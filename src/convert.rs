//! Conversions that do not belong to an either party, but rather to both of them.

use crate::api::wargaming;
use crate::database;
use chrono::Utc;

impl From<&wargaming::models::AccountInfo> for database::models::Account {
    fn from(account_info: &wargaming::models::AccountInfo) -> Self {
        database::models::Account {
            id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            updated_at: Utc::now().into(),
        }
    }
}

impl From<&wargaming::models::AccountInfo> for database::models::AccountSnapshot {
    fn from(account_info: &wargaming::models::AccountInfo) -> Self {
        let statistics = &account_info.statistics.all;
        database::models::AccountSnapshot {
            account_id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            statistics: database::models::AccountSnapshotStatistics {
                battles: statistics.battles,
                survived_battles: statistics.survived_battles,
                wins: statistics.wins,
                win_and_survived: statistics.win_and_survived,
                damage_dealt: statistics.damage_dealt,
                damage_received: statistics.damage_received,
            },
        }
    }
}
