//! Conversions that do not belong to an either party, but rather to both of them.

use chrono::Utc;

use crate::database;
use crate::wargaming;

impl From<wargaming::models::AccountInfo> for database::models::AccountInfo {
    fn from(account_info: wargaming::models::AccountInfo) -> Self {
        Self(
            database::models::Account {
                id: account_info.id,
                nickname: account_info.nickname,
                last_battle_time: account_info.last_battle_time.into(),
                updated_at: Utc::now().into(),
                created_at: account_info.created_at.into(),
            },
            database::models::AccountSnapshot {
                account_id: account_info.id,
                last_battle_time: account_info.last_battle_time.into(),
                statistics: account_info.statistics.all.into(),
            },
        )
    }
}

impl From<database::models::AccountInfo> for wargaming::models::AccountInfo {
    fn from(account_info: database::models::AccountInfo) -> Self {
        let database::models::AccountInfo(account, account_snapshot) = account_info;
        Self {
            id: account.id,
            nickname: account.nickname,
            last_battle_time: account.last_battle_time.into(),
            created_at: account.created_at.into(),
            statistics: wargaming::models::AccountInfoStatistics {
                all: account_snapshot.statistics.into(),
            },
        }
    }
}

impl From<wargaming::models::Statistics> for database::models::StatisticsSnapshot {
    fn from(statistics: wargaming::models::Statistics) -> Self {
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

impl From<database::models::StatisticsSnapshot> for wargaming::models::Statistics {
    fn from(statistics: database::models::StatisticsSnapshot) -> Self {
        Self {
            battles: statistics.battles,
            wins: statistics.wins,
            win_and_survived: statistics.win_and_survived,
            damage_received: statistics.damage_received,
            damage_dealt: statistics.damage_dealt,
            survived_battles: statistics.survived_battles,
        }
    }
}

impl From<wargaming::models::TankStatistics> for database::models::TankSnapshot {
    fn from(statistics: wargaming::models::TankStatistics) -> Self {
        Self {
            account_id: statistics.account_id,
            tank_id: statistics.tank_id,
            last_battle_time: statistics.last_battle_time.into(),
            battle_life_time: statistics.battle_life_time.num_seconds(),
            statistics: statistics.all.into(),
        }
    }
}
