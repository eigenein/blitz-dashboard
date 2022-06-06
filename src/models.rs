use std::collections::HashMap;
use std::ops::Sub;

use itertools::{merge_join_by, EitherOrBoth};
use serde::{Deserialize, Serialize};

use crate::wargaming::models::tank_statistics::{TankAchievements, TankStatistics};
use crate::wargaming::models::{Statistics, TankId};

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
        self.statistics.base.tank_id
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

impl Sub for TankStatistics {
    type Output = TankStatistics;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            base: self.base,
            battle_life_time: self.battle_life_time - rhs.battle_life_time,
            all: self.all - rhs.all,
        }
    }
}

impl Sub for TankAchievements {
    type Output = TankAchievements;

    fn sub(self, _rhs: Self) -> Self::Output {
        Self::Output {
            tank_id: self.tank_id,
            achievements: Default::default(), // TODO
            max_series: Default::default(),   // TODO
        }
    }
}

impl Sub for Statistics {
    type Output = Statistics;

    #[must_use]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            battles: self.battles - rhs.battles,
            wins: self.wins - rhs.wins,
            survived_battles: self.survived_battles - rhs.survived_battles,
            win_and_survived: self.win_and_survived - rhs.win_and_survived,
            damage_dealt: self.damage_dealt - rhs.damage_dealt,
            damage_received: self.damage_received - rhs.damage_received,
            shots: self.shots - rhs.shots,
            hits: self.hits - rhs.hits,
            frags: self.frags - rhs.frags,
            xp: self.xp - rhs.xp,
        }
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

/// Merges tank statistics and tank achievements into a single tank structure.
pub fn merge_tanks(
    account_id: i32,
    mut statistics: Vec<TankStatistics>,
    mut achievements: Vec<TankAchievements>,
) -> Vec<Tank> {
    statistics.sort_unstable_by_key(|snapshot| snapshot.base.tank_id);
    achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

    merge_join_by(statistics, achievements, |left, right| left.base.tank_id.cmp(&right.tank_id))
        .filter_map(|item| match item {
            EitherOrBoth::Both(statistics, achievements) => Some(Tank {
                account_id,
                statistics,
                achievements,
            }),
            _ => None,
        })
        .collect()
}

pub fn subtract_tanks(left: Vec<Tank>, mut right: HashMap<TankId, Tank>) -> Vec<Tank> {
    left.into_iter()
        .filter_map(|left_tank| match right.remove(&left_tank.statistics.base.tank_id) {
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
