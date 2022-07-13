use std::ops::Sub;

use crate::wargaming::{BasicStats, TankId};
use crate::{database, wargaming, AHashMap};

pub fn subtract_tanks(
    realm: wargaming::Realm,
    mut actual_tanks: AHashMap<TankId, database::TankSnapshot>,
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
            (actual_tank.stats.n_battles != snapshot.stats.n_battles).then(|| {
                database::TankSnapshot {
                    realm,
                    last_battle_time: actual_tank.last_battle_time,
                    account_id: snapshot.account_id,
                    tank_id: snapshot.tank_id,
                    battle_life_time: actual_tank.battle_life_time - snapshot.battle_life_time,
                    stats: actual_tank.stats - snapshot.stats,
                }
            })
        })
        .collect();
    subtracted.extend(
        actual_tanks
            .into_values()
            .filter(|tank| tank.stats.n_battles != 0),
    );
    subtracted
}

impl Sub<database::RandomStatsSnapshot> for BasicStats {
    type Output = database::RandomStatsSnapshot;

    fn sub(self, rhs: database::RandomStatsSnapshot) -> Self::Output {
        Self::Output {
            n_battles: self.n_battles - rhs.n_battles,
            n_wins: self.n_wins - rhs.n_wins,
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
