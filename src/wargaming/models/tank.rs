use std::collections::HashMap;
use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::database;
use crate::wargaming::{AccountId, BasicStatistics, TankAchievements, TankId, TankStatistics};

/// Represents a state of a specific player's tank at a specific moment in time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tank {
    pub account_id: AccountId,
    pub statistics: TankStatistics,
    pub achievements: TankAchievements,
}

impl Tank {
    #[must_use]
    pub fn tank_id(&self) -> TankId {
        self.statistics.tank_id
    }
}

pub fn subtract_tanks(
    left: Vec<Tank>,
    mut right: HashMap<TankId, database::TankSnapshot>,
) -> Vec<database::TankSnapshot> {
    left.into_iter()
        .filter_map(|left_tank| match right.remove(&left_tank.statistics.tank_id) {
            Some(right_tank)
                if left_tank.statistics.all.battles > right_tank.statistics.n_battles =>
            {
                Some(database::TankSnapshot {
                    last_battle_time: left_tank.statistics.last_battle_time,
                    account_id: right_tank.account_id,
                    tank_id: right_tank.tank_id,
                    battle_life_time: left_tank.statistics.battle_life_time
                        - right_tank.battle_life_time,
                    statistics: left_tank.statistics.all - right_tank.statistics,
                })
            }
            None if left_tank.statistics.all.battles != 0 => {
                Some(database::TankSnapshot::from(left_tank))
            }
            _ => None,
        })
        .collect()
}

impl Sub<database::StatisticsSnapshot> for BasicStatistics {
    type Output = database::StatisticsSnapshot;

    fn sub(self, rhs: database::StatisticsSnapshot) -> Self::Output {
        Self::Output {
            n_battles: self.battles - rhs.n_battles,
            n_wins: self.wins - rhs.n_wins,
            n_survived_battles: self.survived_battles - rhs.n_survived_battles,
            n_win_and_survived: self.win_and_survived - rhs.n_win_and_survived,
            damage_dealt: self.damage_dealt - rhs.damage_dealt,
            damage_received: self.damage_received - rhs.damage_received,
            n_shots: self.shots - rhs.n_shots,
            n_hits: self.hits - rhs.n_hits,
            n_frags: self.frags - rhs.n_frags,
            xp: self.xp - rhs.xp,
        }
    }
}
