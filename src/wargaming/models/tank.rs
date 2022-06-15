use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::database::TankSnapshot;
use crate::wargaming::models::{TankAchievements, TankId, TankStatistics};

/// Represents a state of a specific player's tank at a specific moment in time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tank {
    pub account_id: i32,
    pub statistics: TankStatistics,
    pub achievements: TankAchievements,
}

impl Tank {
    #[must_use]
    pub fn tank_id(&self) -> TankId {
        self.statistics.basic.tank_id
    }
}

pub fn subtract_tanks(
    left: Vec<Tank>,
    mut right: HashMap<TankId, TankSnapshot>,
) -> Vec<TankSnapshot> {
    left.into_iter()
        .filter_map(|left_tank| match right.remove(&left_tank.statistics.basic.tank_id) {
            Some(right_tank) if left_tank.statistics.all.battles > right_tank.n_battles => {
                Some(TankSnapshot {
                    last_battle_time: left_tank.statistics.basic.last_battle_time,
                    account_id: right_tank.account_id,
                    tank_id: right_tank.tank_id,
                    battle_life_time: left_tank.statistics.battle_life_time
                        - right_tank.battle_life_time,
                    n_battles: left_tank.statistics.all.battles - right_tank.n_battles,
                    n_wins: left_tank.statistics.all.wins - right_tank.n_wins,
                    n_survived_battles: left_tank.statistics.all.survived_battles
                        - right_tank.n_survived_battles,
                    n_win_and_survived: left_tank.statistics.all.win_and_survived
                        - right_tank.n_win_and_survived,
                    damage_dealt: left_tank.statistics.all.damage_dealt - right_tank.damage_dealt,
                    damage_received: left_tank.statistics.all.damage_received
                        - right_tank.damage_received,
                    n_shots: left_tank.statistics.all.shots - right_tank.n_shots,
                    n_hits: left_tank.statistics.all.hits - right_tank.n_hits,
                    n_frags: left_tank.statistics.all.frags - right_tank.n_frags,
                    xp: left_tank.statistics.all.xp - right_tank.xp,
                })
            }
            None if left_tank.statistics.all.battles != 0 => Some(TankSnapshot::from(left_tank)),
            _ => None,
        })
        .collect()
}
