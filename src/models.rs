use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Sub;

use itertools::{merge_join_by, EitherOrBoth};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::wargaming::models::tank_statistics::{TankAchievements, TankStatistics};
use crate::wargaming::models::{Nation, Statistics, TankId};

/// Represents a generic vehicle from the tankopedia.
#[derive(Deserialize, Clone)]
pub struct Vehicle {
    pub tank_id: u16,
    pub name: Cow<'static, str>,
    pub tier: i32,
    pub is_premium: bool,
    pub nation: Nation,

    #[serde(rename = "type")]
    pub type_: TankType,
}

impl Vehicle {
    /// Creates a fake vehicle instance with the specified ID.
    pub fn new_hardcoded(tank_id: TankId) -> Self {
        Self {
            tank_id,
            name: Cow::Owned(format!("#{}", tank_id)),
            tier: 0,
            is_premium: false,
            type_: TankType::Unknown,
            nation: Nation::from_tank_id(tank_id).unwrap_or(Nation::Other),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum TankType {
    #[serde(rename = "lightTank")]
    Light,

    #[serde(rename = "mediumTank")]
    Medium,

    #[serde(rename = "heavyTank")]
    Heavy,

    #[serde(rename = "AT-SPG")]
    AT,

    #[serde(other)]
    Unknown,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wargaming::models::ResultMap;

    #[test]
    fn tankopedia_ok() -> Result {
        serde_json::from_str::<ResultMap<Vehicle>>(
            // language=json
            r#"{"1649":{"suspensions":[1138],"description":"Неостановимый Дракула возродился, и тьма нависла над миром. Долг зовёт охотника на вампиров Хелсинга вновь встать на защиту Света и дать бой древнему злу. Воплощение Хелсинга — это произведение искусства, инкрустированная защитными орнаментами боевая машина, снаряжённая специально для борьбы с порождениями тьмы. Сдвоенное орудие Хелсинга стреляет два раза автоматически — только так можно остановить полёт Дракулы и одержать победу.\r\nПремиум танк «Хелсинг H0» можно было получить во время игрового события «Ночная охота» в октябре 2016 года.","engines":[17013],"prices_xp":null,"next_tanks":null,"modules_tree":{"1138":{"name":"Helsing type1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1138,"type":"vehicleChassis"},"1139":{"name":"Helsing type1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1139,"type":"vehicleTurret"},"1140":{"name":"85mm Twin X-Barrel mod1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1140,"type":"vehicleGun"},"17013":{"name":"Aether W-20","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":17013,"type":"vehicleEngine"}},"nation":"other","is_premium":true,"images":{"preview":"http:\/\/glossary-ru-static.gcdn.co\/icons\/wotb\/current\/uploaded\/vehicles\/hd_thumbnail\/Van_Helsing.png","normal":"http:\/\/glossary-ru-static.gcdn.co\/icons\/wotb\/current\/uploaded\/vehicles\/hd\/Van_Helsing.png"},"cost":null,"default_profile":{"weight":24880,"profile_id":"1138-1139-1140-17013","firepower":62,"shot_efficiency":67,"gun_id":1140,"signal_range":null,"shells":[{"type":"ARMOR_PIERCING","penetration":170,"damage":200},{"type":"ARMOR_PIERCING_CR","penetration":220,"damage":170},{"type":"HIGH_EXPLOSIVE","penetration":45,"damage":300}],"armor":{"turret":{"front":80,"sides":50,"rear":40},"hull":{"front":60,"sides":40,"rear":40}},"speed_forward":60,"battle_level_range_min":7,"speed_backward":15,"engine":{"tier":8,"fire_chance":0.2,"power":500,"name":"Aether W-20","weight":530},"max_ammo":100,"battle_level_range_max":8,"engine_id":17013,"hp":1000,"is_default":true,"protection":30,"suspension":{"tier":7,"load_limit":27800,"traverse_speed":30,"name":"Helsing type1","weight":6000},"suspension_id":1138,"max_weight":27800,"gun":{"move_down_arc":6,"caliber":85,"name":"85mm Twin X-Barrel mod1","weight":3800,"move_up_arc":15,"fire_rate":11.71,"clip_reload_time":0.25,"dispersion":0.34,"clip_capacity":2,"traverse_speed":43.75,"reload_time":10.0,"tier":8,"aim_time":4.2},"turret_id":1139,"turret":{"name":"Helsing type1","weight":3350,"view_range":240,"traverse_left_arc":180,"hp":200,"traverse_speed":17,"tier":7,"traverse_right_arc":180},"maneuverability":53,"hull_weight":10950,"hull_hp":800},"tier":7,"tank_id":1649,"type":"AT-SPG","guns":[1140],"turrets":[1139],"name":"Helsing"}}"#,
        )?;
        Ok(())
    }
}
