use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::wargaming::{AccountId, BasicStatistics, TankAchievements, TankId, TankStatistics};
use crate::{database, AHashMap};

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
    mut actual_tanks: AHashMap<TankId, Tank>,
    snapshots: Vec<database::TankSnapshot>,
) -> Vec<database::TankSnapshot> {
    let mut subtracted: Vec<database::TankSnapshot> = snapshots
        .into_iter()
        .filter_map(|snapshot| {
            actual_tanks
                .remove(&snapshot.tank_id)
                .map(|actual_tank| (snapshot, actual_tank))
        })
        .filter_map(|(snapshot, actual_tank)| {
            (actual_tank.statistics.all.battles != snapshot.statistics.n_battles).then(|| {
                database::TankSnapshot {
                    last_battle_time: actual_tank.statistics.last_battle_time,
                    account_id: snapshot.account_id,
                    tank_id: snapshot.tank_id,
                    battle_life_time: actual_tank.statistics.battle_life_time
                        - snapshot.battle_life_time,
                    statistics: actual_tank.statistics.all - snapshot.statistics,
                }
            })
        })
        .collect();
    subtracted.extend(actual_tanks.into_values().map(database::TankSnapshot::from));
    subtracted
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
