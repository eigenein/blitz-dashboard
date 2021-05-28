//! Conversions that do not belong to an either party, but rather to both of them.

use chrono::Utc;

use crate::database;
use crate::wargaming;

impl From<&wargaming::models::AccountInfo> for database::models::Account {
    fn from(account_info: &wargaming::models::AccountInfo) -> Self {
        Self {
            id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            updated_at: Utc::now().into(),
        }
    }
}

impl From<&wargaming::models::AccountInfo> for database::models::AccountSnapshot {
    fn from(account_info: &wargaming::models::AccountInfo) -> Self {
        Self {
            account_id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            statistics: (&account_info.statistics.all).into(),
        }
    }
}

impl From<&wargaming::models::Statistics> for database::models::StatisticsSnapshot {
    fn from(statistics: &wargaming::models::Statistics) -> Self {
        Self {
            battles: statistics.battles,
            survived_battles: statistics.survived_battles,
            wins: statistics.wins,
            win_and_survived: statistics.win_and_survived,
            damage_dealt: statistics.damage_dealt,
            damage_received: statistics.damage_received,
        }
    }
}

impl From<&wargaming::models::TankStatistics> for database::models::TankSnapshot {
    fn from(statistics: &wargaming::models::TankStatistics) -> Self {
        Self {
            account_id: statistics.account_id,
            tank_id: statistics.tank_id,
            last_battle_time: statistics.last_battle_time.into(),
            battle_life_time: statistics.battle_life_time.num_seconds(),
            statistics: (&statistics.all).into(),
        }
    }
}
