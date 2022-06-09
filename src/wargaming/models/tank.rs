use std::collections::HashMap;
use std::ops::Sub;

use serde::{Deserialize, Serialize};

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
    pub fn tank_id(&self) -> u16 {
        self.statistics.basic.tank_id
    }

    #[must_use]
    pub fn wins_per_hour(&self) -> f64 {
        self.statistics.all.wins as f64 / self.statistics.battle_life_time.num_seconds() as f64
            * 3600.0
    }

    #[must_use]
    pub fn battles_per_hour(&self) -> f64 {
        self.statistics.all.battles as f64 / self.statistics.battle_life_time.num_seconds() as f64
            * 3600.0
    }

    #[must_use]
    pub fn damage_per_minute(&self) -> f64 {
        self.statistics.all.damage_dealt as f64
            / self.statistics.battle_life_time.num_seconds() as f64
            * 60.0
    }
}

impl Sub for Tank {
    type Output = Tank;

    #[must_use]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            account_id: self.account_id,
            statistics: self.statistics - rhs.statistics,
            achievements: self.achievements - rhs.achievements,
        }
    }
}

pub fn subtract_tanks(left: Vec<Tank>, mut right: HashMap<TankId, Tank>) -> Vec<Tank> {
    left.into_iter()
        .filter_map(|left_tank| match right.remove(&left_tank.statistics.basic.tank_id) {
            Some(right_tank)
                if left_tank.statistics.all.battles > right_tank.statistics.all.battles =>
            {
                Some(left_tank - right_tank)
            }
            None if left_tank.statistics.all.battles != 0 => Some(left_tank),
            _ => None,
        })
        .collect()
}
