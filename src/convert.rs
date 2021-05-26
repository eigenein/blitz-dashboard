//! Conversions that do not belong to an either party, but rather to both of them.

use crate::api::wargaming;
use crate::database;
use chrono::Utc;
use std::borrow::Borrow;

impl<A: Borrow<wargaming::models::AccountInfo>> From<A> for database::models::Account {
    fn from(account_info: A) -> Self {
        let account_info = account_info.borrow();
        database::models::Account {
            id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            updated_at: Utc::now().into(),
        }
    }
}

impl<A: Borrow<wargaming::models::AccountInfo>> From<A> for database::models::AccountSnapshot {
    fn from(account_info: A) -> Self {
        let account_info = account_info.borrow();
        database::models::AccountSnapshot {
            account_id: account_info.id,
            last_battle_time: account_info.last_battle_time.into(),
            statistics: (&account_info.statistics.all).into(),
        }
    }
}

impl<S: Borrow<wargaming::models::Statistics>> From<S> for database::models::StatisticsSnapshot {
    fn from(statistics: S) -> Self {
        let statistics = statistics.borrow();
        database::models::StatisticsSnapshot {
            battles: statistics.battles,
            survived_battles: statistics.survived_battles,
            wins: statistics.wins,
            win_and_survived: statistics.win_and_survived,
            damage_dealt: statistics.damage_dealt,
            damage_received: statistics.damage_received,
        }
    }
}

impl<S: Borrow<wargaming::models::TankStatistics>> From<S> for database::models::TankSnapshot {
    fn from(statistics: S) -> Self {
        let statistics = statistics.borrow();
        database::models::TankSnapshot {
            account_id: statistics.account_id,
            tank_id: statistics.tank_id,
            last_battle_time: statistics.last_battle_time.into(),
            battle_life_time: statistics.battle_life_time.num_seconds(),
            statistics: (&statistics.all).into(),
        }
    }
}
