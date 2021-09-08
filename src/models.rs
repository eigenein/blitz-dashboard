use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::Sum;
use std::ops::Sub;

use chrono::{DateTime, Duration, Utc};
use itertools::{merge_join_by, EitherOrBoth};
use serde::{Deserialize, Serialize};

use crate::statistics::ConfidenceInterval;
use crate::thirdparty::serde::{deserialize_duration_seconds, serialize_duration_seconds};

/// Search accounts item.
#[derive(Deserialize, Debug, PartialEq)]
pub struct FoundAccount {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: i32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct BaseAccountInfo {
    #[serde(rename = "account_id")]
    pub id: i32,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_battle_time: DateTime<Utc>,
}

/// Wargaming.net account information.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct AccountInfo {
    #[serde(flatten)]
    pub base: BaseAccountInfo,

    pub nickname: String,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,

    pub statistics: AccountInfoStatistics,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AccountInfoStatistics {
    #[serde(rename = "all")]
    pub all: Statistics,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct Statistics {
    pub battles: i32,
    pub wins: i32,
    pub survived_battles: i32,
    pub win_and_survived: i32,
    pub damage_dealt: i32,
    pub damage_received: i32,
    pub shots: i32,
    pub hits: i32,
    pub frags: i32,
    pub xp: i32,
}

impl Statistics {
    pub fn damage_per_battle(&self) -> f64 {
        self.damage_dealt as f64 / self.battles as f64
    }

    pub fn current_win_rate(&self) -> f64 {
        self.wins as f64 / self.battles as f64
    }

    pub fn survival_rate(&self) -> f64 {
        self.survived_battles as f64 / self.battles as f64
    }

    pub fn hit_rate(&self) -> f64 {
        self.hits as f64 / self.shots as f64
    }

    pub fn frags_per_battle(&self) -> f64 {
        self.frags as f64 / self.battles as f64
    }

    pub fn true_win_rate(&self) -> ConfidenceInterval {
        ConfidenceInterval::default_wilson_score_interval(self.battles, self.wins)
    }
}

impl Sum for Statistics {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum = Self::default();
        for component in iter {
            sum.battles += component.battles;
            sum.wins += component.wins;
            sum.hits += component.hits;
            sum.shots += component.shots;
            sum.survived_battles += component.survived_battles;
            sum.frags += component.frags;
            sum.xp += component.xp;
            sum.damage_received += component.damage_received;
            sum.damage_dealt += component.damage_dealt;
            sum.win_and_survived += component.win_and_survived;
        }
        sum
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub struct BaseTankStatistics {
    pub tank_id: i32,

    /// The moment in time when the related state is actual.
    /// Every new timestamp produces a new tank snapshot in the database.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_battle_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct TankStatistics {
    #[serde(flatten)]
    pub base: BaseTankStatistics,

    #[serde(
        serialize_with = "serialize_duration_seconds",
        deserialize_with = "deserialize_duration_seconds"
    )]
    pub battle_life_time: Duration,

    #[serde(rename = "all")]
    pub all: Statistics,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TankAchievements {
    pub tank_id: i32,
    pub achievements: HashMap<String, i32>,
    pub max_series: HashMap<String, i32>,
}

/// Represents a generic vehicle from the tankopedia.
#[derive(Deserialize, Clone)]
pub struct Vehicle {
    pub tank_id: i32,
    pub name: Cow<'static, str>,
    pub tier: i32,
    pub is_premium: bool,
    pub nation: Nation,

    #[serde(rename = "type")]
    pub type_: TankType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum Nation {
    #[serde(rename = "ussr")]
    Ussr,

    #[serde(rename = "germany")]
    Germany,

    #[serde(rename = "usa")]
    Usa,

    #[serde(rename = "china")]
    China,

    #[serde(rename = "france")]
    France,

    #[serde(rename = "uk")]
    Uk,

    #[serde(rename = "japan")]
    Japan,

    #[serde(rename = "european")]
    Europe,

    #[serde(other, rename = "other")]
    Other,
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
    pub fn wins_per_hour(&self) -> f64 {
        self.statistics.all.wins as f64 / self.statistics.battle_life_time.num_seconds() as f64
            * 3600.0
    }

    pub fn battles_per_hour(&self) -> f64 {
        self.statistics.all.battles as f64 / self.statistics.battle_life_time.num_seconds() as f64
            * 3600.0
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

impl AccountInfo {
    pub fn is_active(&self) -> bool {
        self.base.last_battle_time > (Utc::now() - Duration::days(365))
    }

    pub fn has_recently_played(&self) -> bool {
        self.base.last_battle_time > (Utc::now() - Duration::hours(1))
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

    merge_join_by(statistics, achievements, |left, right| {
        left.base.tank_id.cmp(&right.tank_id)
    })
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

pub fn subtract_tanks(left: Vec<Tank>, mut right: HashMap<i32, Tank>) -> Vec<Tank> {
    left.into_iter()
        .filter_map(
            |left_tank| match right.remove(&left_tank.statistics.base.tank_id) {
                Some(right_tank)
                    if left_tank.statistics.all.battles >= right_tank.statistics.all.battles =>
                {
                    Some(left_tank - right_tank)
                }
                None if left_tank.statistics.all.battles != 0 => Some(left_tank),
                _ => None,
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    type ResultMap<T> = HashMap<String, Option<T>>;
    type ResultMapVec<T> = ResultMap<Vec<T>>;

    #[test]
    fn account_info_map_ok() -> crate::Result {
        serde_json::from_str::<ResultMap<AccountInfo>>(
            // language=json
            r#"{"5589968":{"statistics":{"clan":{"spotted":0,"max_frags_tank_id":0,"hits":0,"frags":0,"max_xp":0,"max_xp_tank_id":0,"wins":0,"losses":0,"capture_points":0,"battles":0,"damage_dealt":0,"damage_received":0,"max_frags":0,"shots":0,"frags8p":0,"xp":0,"win_and_survived":0,"survived_battles":0,"dropped_capture_points":0},"all":{"spotted":5154,"max_frags_tank_id":20817,"hits":48542,"frags":5259,"max_xp":1917,"max_xp_tank_id":54289,"wins":3425,"losses":2609,"capture_points":4571,"battles":6056,"damage_dealt":6009041,"damage_received":4524728,"max_frags":6,"shots":63538,"frags8p":1231,"xp":4008483,"win_and_survived":2524,"survived_battles":2635,"dropped_capture_points":4415},"frags":null},"account_id":5589968,"created_at":1415225091,"updated_at":1621792747,"private":null,"last_battle_time":1621802244,"nickname":"eigenein"}}"#,
        )?;
        Ok(())
    }

    #[test]
    fn account_info_ok() -> crate::Result {
        let info: AccountInfo = serde_json::from_str(
            // language=json
            r#"{"statistics":{"clan":{"spotted":0,"max_frags_tank_id":0,"hits":0,"frags":0,"max_xp":0,"max_xp_tank_id":0,"wins":0,"losses":0,"capture_points":0,"battles":0,"damage_dealt":0,"damage_received":0,"max_frags":0,"shots":0,"frags8p":0,"xp":0,"win_and_survived":0,"survived_battles":0,"dropped_capture_points":0},"all":{"spotted":5154,"max_frags_tank_id":20817,"hits":48542,"frags":5259,"max_xp":1917,"max_xp_tank_id":54289,"wins":3425,"losses":2609,"capture_points":4571,"battles":6056,"damage_dealt":6009041,"damage_received":4524728,"max_frags":6,"shots":63538,"frags8p":1231,"xp":4008483,"win_and_survived":2524,"survived_battles":2635,"dropped_capture_points":4415},"frags":null},"account_id":5589968,"created_at":1415225091,"updated_at":1621792747,"private":null,"last_battle_time":1621802244,"nickname":"eigenein"}"#,
        )?;
        assert_ne!(info.statistics.all.frags, 0);
        assert_ne!(info.statistics.all.xp, 0);
        Ok(())
    }

    #[test]
    fn tank_statistics_ok() -> crate::Result {
        serde_json::from_str::<ResultMapVec<TankStatistics>>(
            // language=json
            r#"{"5589968":[{"all":{"spotted":31,"hits":375,"frags":27,"max_xp":1492,"wins":9,"losses":26,"capture_points":0,"battles":35,"damage_dealt":50982,"damage_received":49138,"max_frags":3,"shots":450,"frags8p":21,"xp":23859,"win_and_survived":9,"survived_battles":10,"dropped_capture_points":15},"last_battle_time":1621550407,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":6703,"in_garage":null,"tank_id":18769},{"all":{"spotted":2,"hits":52,"frags":4,"max_xp":655,"wins":4,"losses":4,"capture_points":0,"battles":8,"damage_dealt":4129,"damage_received":3049,"max_frags":1,"shots":74,"frags8p":0,"xp":3074,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":15},"last_battle_time":1621985039,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1527,"in_garage":null,"tank_id":8257},{"all":{"spotted":37,"hits":131,"frags":16,"max_xp":675,"wins":12,"losses":12,"capture_points":2,"battles":25,"damage_dealt":6184,"damage_received":8308,"max_frags":4,"shots":188,"frags8p":0,"xp":5573,"win_and_survived":7,"survived_battles":7,"dropped_capture_points":2},"last_battle_time":1499024193,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":3382,"in_garage":null,"tank_id":769},{"all":{"spotted":16,"hits":111,"frags":14,"max_xp":1072,"wins":6,"losses":9,"capture_points":0,"battles":15,"damage_dealt":14451,"damage_received":10862,"max_frags":3,"shots":137,"frags8p":0,"xp":8936,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":0},"last_battle_time":1616621162,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2176,"in_garage":null,"tank_id":3713},{"all":{"spotted":11,"hits":51,"frags":4,"max_xp":1177,"wins":4,"losses":8,"capture_points":0,"battles":12,"damage_dealt":15050,"damage_received":16464,"max_frags":1,"shots":62,"frags8p":2,"xp":6804,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":0},"last_battle_time":1621356434,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1832,"in_garage":null,"tank_id":2849},{"all":{"spotted":13,"hits":43,"frags":5,"max_xp":1011,"wins":3,"losses":4,"capture_points":0,"battles":7,"damage_dealt":4798,"damage_received":4929,"max_frags":2,"shots":52,"frags8p":0,"xp":3392,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1621981072,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":956,"in_garage":null,"tank_id":19713},{"all":{"spotted":20,"hits":297,"frags":48,"max_xp":1006,"wins":40,"losses":15,"capture_points":117,"battles":55,"damage_dealt":34889,"damage_received":16548,"max_frags":3,"shots":387,"frags8p":0,"xp":27527,"win_and_survived":34,"survived_battles":34,"dropped_capture_points":265},"last_battle_time":1621791663,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":9420,"in_garage":null,"tank_id":52065},{"all":{"spotted":18,"hits":94,"frags":12,"max_xp":1219,"wins":5,"losses":6,"capture_points":0,"battles":11,"damage_dealt":10359,"damage_received":6143,"max_frags":4,"shots":106,"frags8p":0,"xp":6793,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1615810656,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":1822,"in_garage":null,"tank_id":51473},{"all":{"spotted":21,"hits":129,"frags":16,"max_xp":1116,"wins":9,"losses":10,"capture_points":0,"battles":19,"damage_dealt":13775,"damage_received":13149,"max_frags":3,"shots":168,"frags8p":0,"xp":11235,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":63},"last_battle_time":1615731215,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2964,"in_garage":null,"tank_id":5745},{"all":{"spotted":4,"hits":80,"frags":8,"max_xp":497,"wins":5,"losses":4,"capture_points":135,"battles":9,"damage_dealt":1507,"damage_received":698,"max_frags":3,"shots":216,"frags8p":0,"xp":2181,"win_and_survived":5,"survived_battles":6,"dropped_capture_points":0},"last_battle_time":1416428862,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1987,"in_garage":null,"tank_id":3329},{"all":{"spotted":53,"hits":521,"frags":84,"max_xp":1585,"wins":64,"losses":82,"capture_points":0,"battles":146,"damage_dealt":134212,"damage_received":99023,"max_frags":5,"shots":727,"frags8p":9,"xp":69452,"win_and_survived":54,"survived_battles":64,"dropped_capture_points":56},"last_battle_time":1614118659,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":29935,"in_garage":null,"tank_id":55297},{"all":{"spotted":11,"hits":61,"frags":4,"max_xp":375,"wins":4,"losses":4,"capture_points":0,"battles":8,"damage_dealt":2029,"damage_received":2080,"max_frags":2,"shots":74,"frags8p":0,"xp":1741,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1500494749,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1025,"in_garage":null,"tank_id":289},{"all":{"spotted":24,"hits":215,"frags":9,"max_xp":448,"wins":5,"losses":8,"capture_points":185,"battles":13,"damage_dealt":2837,"damage_received":1814,"max_frags":3,"shots":366,"frags8p":0,"xp":2177,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":168},"last_battle_time":1416434534,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2096,"in_garage":null,"tank_id":545},{"all":{"spotted":3,"hits":93,"frags":14,"max_xp":1000,"wins":4,"losses":4,"capture_points":0,"battles":8,"damage_dealt":5965,"damage_received":4486,"max_frags":3,"shots":106,"frags8p":0,"xp":4647,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1621985632,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1432,"in_garage":null,"tank_id":2913},{"all":{"spotted":26,"hits":158,"frags":19,"max_xp":997,"wins":17,"losses":9,"capture_points":307,"battles":26,"damage_dealt":8990,"damage_received":7309,"max_frags":4,"shots":236,"frags8p":0,"xp":7606,"win_and_survived":11,"survived_battles":11,"dropped_capture_points":0},"last_battle_time":1619448284,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":3945,"in_garage":null,"tank_id":4881},{"all":{"spotted":163,"hits":434,"frags":54,"max_xp":1708,"wins":40,"losses":42,"capture_points":0,"battles":82,"damage_dealt":100140,"damage_received":128555,"max_frags":3,"shots":499,"frags8p":54,"xp":65807,"win_and_survived":21,"survived_battles":22,"dropped_capture_points":0},"last_battle_time":1621119737,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":11460,"in_garage":null,"tank_id":3649},{"all":{"spotted":120,"hits":745,"frags":74,"max_xp":1675,"wins":55,"losses":42,"capture_points":20,"battles":98,"damage_dealt":115629,"damage_received":93501,"max_frags":5,"shots":901,"frags8p":49,"xp":79487,"win_and_survived":37,"survived_battles":41,"dropped_capture_points":90},"last_battle_time":1618069881,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":18010,"in_garage":null,"tank_id":20769},{"all":{"spotted":5,"hits":65,"frags":17,"max_xp":1141,"wins":6,"losses":5,"capture_points":3,"battles":11,"damage_dealt":9877,"damage_received":7227,"max_frags":3,"shots":79,"frags8p":0,"xp":6455,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":0},"last_battle_time":1621982645,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":2072,"in_garage":null,"tank_id":16145},{"all":{"spotted":225,"hits":1889,"frags":213,"max_xp":1917,"wins":145,"losses":110,"capture_points":200,"battles":257,"damage_dealt":401299,"damage_received":337566,"max_frags":5,"shots":2238,"frags8p":166,"xp":232484,"win_and_survived":119,"survived_battles":122,"dropped_capture_points":91},"last_battle_time":1621362085,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":49387,"in_garage":null,"tank_id":54289},{"all":{"spotted":20,"hits":43,"frags":6,"max_xp":384,"wins":2,"losses":5,"capture_points":0,"battles":7,"damage_dealt":1596,"damage_received":1262,"max_frags":3,"shots":61,"frags8p":0,"xp":1357,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1500236282,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":845,"in_garage":null,"tank_id":1825},{"all":{"spotted":61,"hits":217,"frags":40,"max_xp":1128,"wins":29,"losses":14,"capture_points":10,"battles":44,"damage_dealt":30047,"damage_received":21957,"max_frags":3,"shots":278,"frags8p":0,"xp":25226,"win_and_survived":21,"survived_battles":21,"dropped_capture_points":61},"last_battle_time":1616108925,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":7426,"in_garage":null,"tank_id":4977},{"all":{"spotted":8,"hits":98,"frags":12,"max_xp":1431,"wins":5,"losses":5,"capture_points":9,"battles":10,"damage_dealt":11692,"damage_received":7895,"max_frags":3,"shots":119,"frags8p":0,"xp":7568,"win_and_survived":5,"survived_battles":6,"dropped_capture_points":0},"last_battle_time":1622026002,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1648,"in_garage":null,"tank_id":6257},{"all":{"spotted":42,"hits":983,"frags":20,"max_xp":576,"wins":11,"losses":14,"capture_points":66,"battles":25,"damage_dealt":7396,"damage_received":4846,"max_frags":4,"shots":1486,"frags8p":0,"xp":6596,"win_and_survived":8,"survived_battles":9,"dropped_capture_points":93},"last_battle_time":1479653775,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":4376,"in_garage":null,"tank_id":2065},{"all":{"spotted":10,"hits":116,"frags":6,"max_xp":678,"wins":4,"losses":4,"capture_points":0,"battles":8,"damage_dealt":2693,"damage_received":3188,"max_frags":2,"shots":143,"frags8p":0,"xp":2314,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1502834041,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1185,"in_garage":null,"tank_id":5153},{"all":{"spotted":0,"hits":16,"frags":2,"max_xp":457,"wins":1,"losses":0,"capture_points":0,"battles":1,"damage_dealt":327,"damage_received":49,"max_frags":2,"shots":20,"frags8p":0,"xp":457,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1522666426,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":242,"in_garage":null,"tank_id":1329},{"all":{"spotted":24,"hits":105,"frags":6,"max_xp":357,"wins":12,"losses":6,"capture_points":75,"battles":19,"damage_dealt":3254,"damage_received":5316,"max_frags":1,"shots":155,"frags8p":0,"xp":3852,"win_and_survived":4,"survived_battles":5,"dropped_capture_points":41},"last_battle_time":1474039708,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":2726,"in_garage":null,"tank_id":4897},{"all":{"spotted":40,"hits":277,"frags":28,"max_xp":1291,"wins":25,"losses":27,"capture_points":35,"battles":52,"damage_dealt":40434,"damage_received":43466,"max_frags":2,"shots":331,"frags8p":1,"xp":24951,"win_and_survived":19,"survived_battles":19,"dropped_capture_points":0},"last_battle_time":1620475149,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":8119,"in_garage":null,"tank_id":63585},{"all":{"spotted":23,"hits":98,"frags":11,"max_xp":778,"wins":10,"losses":8,"capture_points":42,"battles":18,"damage_dealt":11513,"damage_received":8937,"max_frags":2,"shots":123,"frags8p":0,"xp":8542,"win_and_survived":5,"survived_battles":5,"dropped_capture_points":0},"last_battle_time":1615471754,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":2336,"in_garage":null,"tank_id":1921},{"all":{"spotted":3,"hits":9,"frags":2,"max_xp":361,"wins":2,"losses":1,"capture_points":100,"battles":3,"damage_dealt":621,"damage_received":283,"max_frags":1,"shots":13,"frags8p":0,"xp":706,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":1},"last_battle_time":1474020170,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":493,"in_garage":null,"tank_id":81},{"all":{"spotted":74,"hits":686,"frags":86,"max_xp":1608,"wins":60,"losses":30,"capture_points":6,"battles":91,"damage_dealt":76541,"damage_received":51559,"max_frags":4,"shots":836,"frags8p":0,"xp":63649,"win_and_survived":43,"survived_battles":43,"dropped_capture_points":173},"last_battle_time":1621713014,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":14238,"in_garage":null,"tank_id":51457},{"all":{"spotted":184,"hits":531,"frags":72,"max_xp":1860,"wins":51,"losses":55,"capture_points":23,"battles":107,"damage_dealt":124715,"damage_received":163046,"max_frags":4,"shots":610,"frags8p":72,"xp":80869,"win_and_survived":24,"survived_battles":25,"dropped_capture_points":1},"last_battle_time":1621119810,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":13663,"in_garage":null,"tank_id":1857},{"all":{"spotted":1,"hits":49,"frags":9,"max_xp":781,"wins":8,"losses":3,"capture_points":0,"battles":11,"damage_dealt":8227,"damage_received":3549,"max_frags":2,"shots":72,"frags8p":0,"xp":5800,"win_and_survived":7,"survived_battles":7,"dropped_capture_points":0},"last_battle_time":1621785448,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":2178,"in_garage":null,"tank_id":7537},{"all":{"spotted":133,"hits":1272,"frags":169,"max_xp":1543,"wins":108,"losses":78,"capture_points":257,"battles":187,"damage_dealt":120518,"damage_received":101635,"max_frags":5,"shots":1526,"frags8p":0,"xp":99280,"win_and_survived":85,"survived_battles":89,"dropped_capture_points":248},"last_battle_time":1617909880,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":33762,"in_garage":null,"tank_id":51201},{"all":{"spotted":21,"hits":73,"frags":5,"max_xp":1011,"wins":4,"losses":9,"capture_points":0,"battles":13,"damage_dealt":10736,"damage_received":10805,"max_frags":2,"shots":93,"frags8p":0,"xp":6273,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1620936567,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1734,"in_garage":null,"tank_id":22785},{"all":{"spotted":8,"hits":148,"frags":14,"max_xp":1089,"wins":5,"losses":5,"capture_points":98,"battles":11,"damage_dealt":9059,"damage_received":4855,"max_frags":3,"shots":167,"frags8p":0,"xp":5982,"win_and_survived":4,"survived_battles":5,"dropped_capture_points":22},"last_battle_time":1617828188,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2271,"in_garage":null,"tank_id":8209},{"all":{"spotted":40,"hits":216,"frags":31,"max_xp":1338,"wins":20,"losses":14,"capture_points":107,"battles":34,"damage_dealt":22879,"damage_received":20139,"max_frags":5,"shots":270,"frags8p":0,"xp":18778,"win_and_survived":14,"survived_battles":14,"dropped_capture_points":64},"last_battle_time":1618700019,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":5432,"in_garage":null,"tank_id":54545},{"all":{"spotted":65,"hits":443,"frags":55,"max_xp":1120,"wins":43,"losses":43,"capture_points":113,"battles":87,"damage_dealt":46781,"damage_received":46623,"max_frags":3,"shots":562,"frags8p":0,"xp":31678,"win_and_survived":32,"survived_battles":33,"dropped_capture_points":90},"last_battle_time":1618947252,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":13916,"in_garage":null,"tank_id":63841},{"all":{"spotted":13,"hits":128,"frags":12,"max_xp":1134,"wins":5,"losses":6,"capture_points":0,"battles":11,"damage_dealt":5056,"damage_received":3729,"max_frags":4,"shots":160,"frags8p":0,"xp":4611,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":103},"last_battle_time":1617827863,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":1801,"in_garage":null,"tank_id":53537},{"all":{"spotted":18,"hits":101,"frags":7,"max_xp":1567,"wins":5,"losses":12,"capture_points":0,"battles":17,"damage_dealt":22785,"damage_received":30359,"max_frags":3,"shots":129,"frags8p":7,"xp":10739,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1620684654,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":2544,"in_garage":null,"tank_id":385},{"all":{"spotted":14,"hits":60,"frags":5,"max_xp":765,"wins":3,"losses":6,"capture_points":0,"battles":9,"damage_dealt":6037,"damage_received":4639,"max_frags":2,"shots":80,"frags8p":0,"xp":3630,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1621983591,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1316,"in_garage":null,"tank_id":5489},{"all":{"spotted":21,"hits":82,"frags":19,"max_xp":753,"wins":10,"losses":4,"capture_points":106,"battles":14,"damage_dealt":4161,"damage_received":2179,"max_frags":6,"shots":96,"frags8p":0,"xp":3679,"win_and_survived":5,"survived_battles":5,"dropped_capture_points":0},"last_battle_time":1418416756,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":1936,"in_garage":null,"tank_id":6177},{"all":{"spotted":8,"hits":23,"frags":5,"max_xp":288,"wins":5,"losses":5,"capture_points":37,"battles":11,"damage_dealt":1457,"damage_received":1651,"max_frags":2,"shots":39,"frags8p":0,"xp":1379,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":6},"last_battle_time":1420940972,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1462,"in_garage":null,"tank_id":5121},{"all":{"spotted":36,"hits":330,"frags":43,"max_xp":1438,"wins":25,"losses":17,"capture_points":13,"battles":42,"damage_dealt":43444,"damage_received":33633,"max_frags":4,"shots":410,"frags8p":0,"xp":30347,"win_and_survived":13,"survived_battles":14,"dropped_capture_points":0},"last_battle_time":1620844580,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":7468,"in_garage":null,"tank_id":9009},{"all":{"spotted":8,"hits":92,"frags":2,"max_xp":651,"wins":2,"losses":5,"capture_points":0,"battles":7,"damage_dealt":1632,"damage_received":1698,"max_frags":1,"shots":129,"frags8p":0,"xp":1329,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1420221904,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":943,"in_garage":null,"tank_id":785},{"all":{"spotted":5,"hits":85,"frags":8,"max_xp":991,"wins":6,"losses":12,"capture_points":0,"battles":18,"damage_dealt":11395,"damage_received":8536,"max_frags":2,"shots":111,"frags8p":0,"xp":6593,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":0},"last_battle_time":1621712402,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2829,"in_garage":null,"tank_id":64593},{"all":{"spotted":6,"hits":80,"frags":13,"max_xp":1158,"wins":5,"losses":3,"capture_points":0,"battles":8,"damage_dealt":8294,"damage_received":3191,"max_frags":3,"shots":91,"frags8p":0,"xp":6009,"win_and_survived":5,"survived_battles":5,"dropped_capture_points":0},"last_battle_time":1621982418,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1342,"in_garage":null,"tank_id":52257},{"all":{"spotted":832,"hits":14626,"frags":1629,"max_xp":1839,"wins":1011,"losses":553,"capture_points":365,"battles":1565,"damage_dealt":1929645,"damage_received":1030241,"max_frags":5,"shots":19763,"frags8p":384,"xp":1297656,"win_and_survived":816,"survived_battles":842,"dropped_capture_points":173},"last_battle_time":1620505056,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":286577,"in_garage":null,"tank_id":1649},{"all":{"spotted":16,"hits":100,"frags":5,"max_xp":781,"wins":4,"losses":8,"capture_points":34,"battles":13,"damage_dealt":5762,"damage_received":5735,"max_frags":1,"shots":125,"frags8p":0,"xp":4422,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":66},"last_battle_time":1561893168,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2436,"in_garage":null,"tank_id":5409},{"all":{"spotted":24,"hits":221,"frags":11,"max_xp":922,"wins":11,"losses":6,"capture_points":0,"battles":17,"damage_dealt":8553,"damage_received":6613,"max_frags":3,"shots":297,"frags8p":0,"xp":6613,"win_and_survived":8,"survived_battles":8,"dropped_capture_points":80},"last_battle_time":1501259362,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2643,"in_garage":null,"tank_id":2049},{"all":{"spotted":7,"hits":71,"frags":8,"max_xp":1422,"wins":6,"losses":6,"capture_points":0,"battles":12,"damage_dealt":15487,"damage_received":16505,"max_frags":2,"shots":87,"frags8p":8,"xp":9238,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1616620232,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1702,"in_garage":null,"tank_id":641},{"all":{"spotted":1,"hits":18,"frags":2,"max_xp":891,"wins":4,"losses":0,"capture_points":0,"battles":4,"damage_dealt":2780,"damage_received":2151,"max_frags":1,"shots":21,"frags8p":1,"xp":3096,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1577638782,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":730,"in_garage":null,"tank_id":11297},{"all":{"spotted":5,"hits":15,"frags":4,"max_xp":1036,"wins":4,"losses":4,"capture_points":0,"battles":8,"damage_dealt":4610,"damage_received":5558,"max_frags":1,"shots":18,"frags8p":0,"xp":3789,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1622025328,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1169,"in_garage":null,"tank_id":54785},{"all":{"spotted":0,"hits":55,"frags":0,"max_xp":291,"wins":1,"losses":0,"capture_points":71,"battles":1,"damage_dealt":172,"damage_received":71,"max_frags":0,"shots":75,"frags8p":0,"xp":291,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1496015279,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":210,"in_garage":null,"tank_id":577},{"all":{"spotted":39,"hits":312,"frags":31,"max_xp":781,"wins":16,"losses":17,"capture_points":57,"battles":35,"damage_dealt":14107,"damage_received":11805,"max_frags":3,"shots":376,"frags8p":0,"xp":10835,"win_and_survived":11,"survived_battles":11,"dropped_capture_points":97},"last_battle_time":1615120615,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":5509,"in_garage":null,"tank_id":51985},{"all":{"spotted":19,"hits":171,"frags":13,"max_xp":551,"wins":5,"losses":4,"capture_points":0,"battles":9,"damage_dealt":3392,"damage_received":1918,"max_frags":4,"shots":275,"frags8p":0,"xp":2504,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":6},"last_battle_time":1417143036,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":1576,"in_garage":null,"tank_id":5665},{"all":{"spotted":34,"hits":537,"frags":26,"max_xp":1101,"wins":14,"losses":6,"capture_points":9,"battles":20,"damage_dealt":12340,"damage_received":8339,"max_frags":5,"shots":792,"frags8p":0,"xp":11078,"win_and_survived":10,"survived_battles":10,"dropped_capture_points":264},"last_battle_time":1615122152,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":3164,"in_garage":null,"tank_id":6161},{"all":{"spotted":11,"hits":84,"frags":2,"max_xp":498,"wins":6,"losses":4,"capture_points":22,"battles":10,"damage_dealt":2232,"damage_received":3972,"max_frags":1,"shots":100,"frags8p":0,"xp":2988,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1614980323,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1479,"in_garage":null,"tank_id":52737},{"all":{"spotted":7,"hits":136,"frags":17,"max_xp":915,"wins":4,"losses":9,"capture_points":0,"battles":13,"damage_dealt":9581,"damage_received":7194,"max_frags":2,"shots":167,"frags8p":0,"xp":6534,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1617828087,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":2648,"in_garage":null,"tank_id":4945},{"all":{"spotted":34,"hits":205,"frags":47,"max_xp":802,"wins":24,"losses":21,"capture_points":49,"battles":48,"damage_dealt":21635,"damage_received":15512,"max_frags":5,"shots":262,"frags8p":0,"xp":13105,"win_and_survived":16,"survived_battles":18,"dropped_capture_points":69},"last_battle_time":1494716812,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":8386,"in_garage":null,"tank_id":7713},{"all":{"spotted":3,"hits":53,"frags":13,"max_xp":826,"wins":4,"losses":6,"capture_points":51,"battles":10,"damage_dealt":4374,"damage_received":5029,"max_frags":5,"shots":63,"frags8p":0,"xp":3942,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1621984746,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1552,"in_garage":null,"tank_id":2881},{"all":{"spotted":18,"hits":57,"frags":12,"max_xp":569,"wins":10,"losses":6,"capture_points":0,"battles":16,"damage_dealt":4625,"damage_received":3075,"max_frags":3,"shots":80,"frags8p":0,"xp":3440,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":2},"last_battle_time":1474040141,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1982,"in_garage":null,"tank_id":6433},{"all":{"spotted":5,"hits":28,"frags":4,"max_xp":1075,"wins":2,"losses":3,"capture_points":0,"battles":5,"damage_dealt":5328,"damage_received":5565,"max_frags":2,"shots":31,"frags8p":4,"xp":3047,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1618767007,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":0,"battle_life_time":710,"in_garage":null,"tank_id":3457},{"all":{"spotted":91,"hits":627,"frags":105,"max_xp":1548,"wins":69,"losses":59,"capture_points":39,"battles":128,"damage_dealt":154885,"damage_received":133470,"max_frags":4,"shots":764,"frags8p":30,"xp":92866,"win_and_survived":58,"survived_battles":59,"dropped_capture_points":77},"last_battle_time":1621455394,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":24089,"in_garage":null,"tank_id":3697},{"all":{"spotted":11,"hits":130,"frags":30,"max_xp":1417,"wins":12,"losses":13,"capture_points":2,"battles":25,"damage_dealt":29460,"damage_received":17647,"max_frags":5,"shots":159,"frags8p":0,"xp":16678,"win_and_survived":7,"survived_battles":7,"dropped_capture_points":0},"last_battle_time":1618765195,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":4513,"in_garage":null,"tank_id":57105},{"all":{"spotted":107,"hits":824,"frags":87,"max_xp":1471,"wins":57,"losses":42,"capture_points":0,"battles":99,"damage_dealt":92324,"damage_received":75129,"max_frags":3,"shots":1023,"frags8p":0,"xp":71707,"win_and_survived":42,"survived_battles":46,"dropped_capture_points":91},"last_battle_time":1620934869,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":15947,"in_garage":null,"tank_id":21281},{"all":{"spotted":5,"hits":154,"frags":8,"max_xp":452,"wins":10,"losses":0,"capture_points":37,"battles":10,"damage_dealt":1525,"damage_received":1026,"max_frags":2,"shots":368,"frags8p":0,"xp":2813,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":35},"last_battle_time":1476167847,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":1604,"in_garage":null,"tank_id":3089},{"all":{"spotted":9,"hits":52,"frags":11,"max_xp":682,"wins":6,"losses":3,"capture_points":26,"battles":9,"damage_dealt":4472,"damage_received":3541,"max_frags":3,"shots":62,"frags8p":0,"xp":3350,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":23},"last_battle_time":1561810993,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1444,"in_garage":null,"tank_id":17425},{"all":{"spotted":46,"hits":511,"frags":63,"max_xp":1476,"wins":45,"losses":34,"capture_points":10,"battles":79,"damage_dealt":104230,"damage_received":67379,"max_frags":3,"shots":608,"frags8p":12,"xp":59423,"win_and_survived":36,"survived_battles":36,"dropped_capture_points":44},"last_battle_time":1621453318,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":13415,"in_garage":null,"tank_id":56609},{"all":{"spotted":8,"hits":39,"frags":8,"max_xp":597,"wins":4,"losses":2,"capture_points":18,"battles":6,"damage_dealt":2018,"damage_received":1046,"max_frags":3,"shots":46,"frags8p":0,"xp":1735,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1420222289,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":840,"in_garage":null,"tank_id":3601},{"all":{"spotted":15,"hits":46,"frags":14,"max_xp":484,"wins":8,"losses":4,"capture_points":9,"battles":12,"damage_dealt":3673,"damage_received":4088,"max_frags":3,"shots":62,"frags8p":0,"xp":3752,"win_and_survived":5,"survived_battles":5,"dropped_capture_points":56},"last_battle_time":1614980525,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1850,"in_garage":null,"tank_id":60929},{"all":{"spotted":5,"hits":37,"frags":5,"max_xp":588,"wins":1,"losses":2,"capture_points":96,"battles":3,"damage_dealt":1586,"damage_received":1204,"max_frags":3,"shots":47,"frags8p":0,"xp":865,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":37},"last_battle_time":1548024233,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":682,"in_garage":null,"tank_id":4369},{"all":{"spotted":150,"hits":1051,"frags":140,"max_xp":1575,"wins":78,"losses":57,"capture_points":12,"battles":135,"damage_dealt":151024,"damage_received":115251,"max_frags":6,"shots":1248,"frags8p":0,"xp":101670,"win_and_survived":55,"survived_battles":62,"dropped_capture_points":4},"last_battle_time":1620935776,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":23649,"in_garage":null,"tank_id":22801},{"all":{"spotted":24,"hits":228,"frags":16,"max_xp":542,"wins":11,"losses":10,"capture_points":68,"battles":21,"damage_dealt":8941,"damage_received":6753,"max_frags":3,"shots":378,"frags8p":0,"xp":6504,"win_and_survived":8,"survived_battles":8,"dropped_capture_points":269},"last_battle_time":1500325024,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":3255,"in_garage":null,"tank_id":3073},{"all":{"spotted":26,"hits":298,"frags":15,"max_xp":979,"wins":12,"losses":9,"capture_points":439,"battles":21,"damage_dealt":10195,"damage_received":7375,"max_frags":2,"shots":369,"frags8p":0,"xp":10495,"win_and_survived":10,"survived_battles":11,"dropped_capture_points":144},"last_battle_time":1562622366,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":4526,"in_garage":null,"tank_id":1089},{"all":{"spotted":2,"hits":68,"frags":2,"max_xp":568,"wins":1,"losses":1,"capture_points":2,"battles":2,"damage_dealt":543,"damage_received":450,"max_frags":1,"shots":125,"frags8p":0,"xp":785,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1420226068,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":501,"in_garage":null,"tank_id":4609},{"all":{"spotted":7,"hits":17,"frags":3,"max_xp":655,"wins":2,"losses":2,"capture_points":0,"battles":4,"damage_dealt":1331,"damage_received":2419,"max_frags":1,"shots":20,"frags8p":0,"xp":1423,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1562929017,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":547,"in_garage":null,"tank_id":14145},{"all":{"spotted":53,"hits":320,"frags":42,"max_xp":1333,"wins":25,"losses":16,"capture_points":18,"battles":41,"damage_dealt":50867,"damage_received":35002,"max_frags":3,"shots":389,"frags8p":0,"xp":31389,"win_and_survived":19,"survived_battles":19,"dropped_capture_points":0},"last_battle_time":1620934948,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":6866,"in_garage":null,"tank_id":22049},{"all":{"spotted":2,"hits":13,"frags":4,"max_xp":781,"wins":1,"losses":0,"capture_points":0,"battles":1,"damage_dealt":582,"damage_received":250,"max_frags":4,"shots":15,"frags8p":0,"xp":781,"win_and_survived":0,"survived_battles":0,"dropped_capture_points":0},"last_battle_time":1420226581,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":159,"in_garage":null,"tank_id":1025},{"all":{"spotted":45,"hits":345,"frags":50,"max_xp":861,"wins":25,"losses":11,"capture_points":224,"battles":36,"damage_dealt":14649,"damage_received":8646,"max_frags":4,"shots":479,"frags8p":0,"xp":13939,"win_and_survived":20,"survived_battles":20,"dropped_capture_points":36},"last_battle_time":1615120190,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":6987,"in_garage":null,"tank_id":56577},{"all":{"spotted":88,"hits":408,"frags":39,"max_xp":1383,"wins":34,"losses":17,"capture_points":0,"battles":51,"damage_dealt":42639,"damage_received":44020,"max_frags":3,"shots":510,"frags8p":11,"xp":38115,"win_and_survived":16,"survived_battles":17,"dropped_capture_points":0},"last_battle_time":1621118693,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":7773,"in_garage":null,"tank_id":5185},{"all":{"spotted":38,"hits":234,"frags":27,"max_xp":1230,"wins":15,"losses":14,"capture_points":0,"battles":29,"damage_dealt":30020,"damage_received":23658,"max_frags":4,"shots":276,"frags8p":0,"xp":19326,"win_and_survived":7,"survived_battles":8,"dropped_capture_points":0},"last_battle_time":1621030963,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":4022,"in_garage":null,"tank_id":57361},{"all":{"spotted":20,"hits":66,"frags":4,"max_xp":557,"wins":4,"losses":5,"capture_points":44,"battles":9,"damage_dealt":1687,"damage_received":2032,"max_frags":3,"shots":101,"frags8p":0,"xp":1680,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":96},"last_battle_time":1420939622,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1235,"in_garage":null,"tank_id":7761},{"all":{"spotted":11,"hits":97,"frags":10,"max_xp":853,"wins":6,"losses":4,"capture_points":0,"battles":10,"damage_dealt":3701,"damage_received":3219,"max_frags":3,"shots":118,"frags8p":0,"xp":4319,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":4},"last_battle_time":1615120834,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1528,"in_garage":null,"tank_id":54801},{"all":{"spotted":42,"hits":305,"frags":25,"max_xp":1702,"wins":18,"losses":14,"capture_points":0,"battles":32,"damage_dealt":42192,"damage_received":35899,"max_frags":3,"shots":348,"frags8p":23,"xp":28208,"win_and_survived":14,"survived_battles":14,"dropped_capture_points":0},"last_battle_time":1621374508,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":8447,"in_garage":null,"tank_id":19025},{"all":{"spotted":131,"hits":771,"frags":147,"max_xp":1723,"wins":105,"losses":63,"capture_points":28,"battles":168,"damage_dealt":197765,"damage_received":161156,"max_frags":4,"shots":931,"frags8p":20,"xp":134645,"win_and_survived":86,"survived_battles":90,"dropped_capture_points":17},"last_battle_time":1621456023,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":31097,"in_garage":null,"tank_id":7281},{"all":{"spotted":14,"hits":94,"frags":16,"max_xp":726,"wins":8,"losses":12,"capture_points":52,"battles":20,"damage_dealt":10019,"damage_received":6344,"max_frags":3,"shots":130,"frags8p":0,"xp":5807,"win_and_survived":8,"survived_battles":8,"dropped_capture_points":81},"last_battle_time":1499018237,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":3601,"in_garage":null,"tank_id":10273},{"all":{"spotted":81,"hits":383,"frags":40,"max_xp":1600,"wins":37,"losses":22,"capture_points":0,"battles":59,"damage_dealt":60207,"damage_received":46327,"max_frags":3,"shots":477,"frags8p":4,"xp":45669,"win_and_survived":28,"survived_battles":29,"dropped_capture_points":0},"last_battle_time":1618778054,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":9792,"in_garage":null,"tank_id":21025},{"all":{"spotted":3,"hits":38,"frags":7,"max_xp":562,"wins":3,"losses":2,"capture_points":100,"battles":5,"damage_dealt":1301,"damage_received":306,"max_frags":3,"shots":56,"frags8p":0,"xp":1332,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1473930040,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":949,"in_garage":null,"tank_id":609},{"all":{"spotted":165,"hits":1224,"frags":109,"max_xp":1689,"wins":85,"losses":89,"capture_points":23,"battles":174,"damage_dealt":196930,"damage_received":190195,"max_frags":4,"shots":1496,"frags8p":91,"xp":134817,"win_and_survived":59,"survived_battles":63,"dropped_capture_points":27},"last_battle_time":1621362605,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":29357,"in_garage":null,"tank_id":13345},{"all":{"spotted":8,"hits":94,"frags":13,"max_xp":1879,"wins":6,"losses":4,"capture_points":0,"battles":10,"damage_dealt":16395,"damage_received":11060,"max_frags":4,"shots":110,"frags8p":7,"xp":10085,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":43},"last_battle_time":1621786074,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":1735,"in_garage":null,"tank_id":2945},{"all":{"spotted":48,"hits":576,"frags":65,"max_xp":883,"wins":64,"losses":65,"capture_points":67,"battles":130,"damage_dealt":65927,"damage_received":53803,"max_frags":3,"shots":728,"frags8p":0,"xp":40748,"win_and_survived":42,"survived_battles":44,"dropped_capture_points":92},"last_battle_time":1498241858,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":21477,"in_garage":null,"tank_id":6945},{"all":{"spotted":25,"hits":408,"frags":66,"max_xp":1492,"wins":33,"losses":33,"capture_points":173,"battles":66,"damage_dealt":88334,"damage_received":72908,"max_frags":3,"shots":488,"frags8p":48,"xp":52224,"win_and_survived":26,"survived_battles":28,"dropped_capture_points":5},"last_battle_time":1621374230,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":12421,"in_garage":null,"tank_id":16193},{"all":{"spotted":22,"hits":1284,"frags":16,"max_xp":951,"wins":12,"losses":12,"capture_points":66,"battles":24,"damage_dealt":9451,"damage_received":7407,"max_frags":4,"shots":1905,"frags8p":0,"xp":9848,"win_and_survived":7,"survived_battles":7,"dropped_capture_points":198},"last_battle_time":1615119983,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":4002,"in_garage":null,"tank_id":55073},{"all":{"spotted":20,"hits":100,"frags":11,"max_xp":877,"wins":10,"losses":7,"capture_points":0,"battles":17,"damage_dealt":11471,"damage_received":8133,"max_frags":3,"shots":124,"frags8p":0,"xp":7566,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":0},"last_battle_time":1577806540,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2618,"in_garage":null,"tank_id":1057},{"all":{"spotted":6,"hits":39,"frags":6,"max_xp":739,"wins":3,"losses":5,"capture_points":0,"battles":8,"damage_dealt":4820,"damage_received":3254,"max_frags":3,"shots":49,"frags8p":0,"xp":3156,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1621983820,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1117,"in_garage":null,"tank_id":3105},{"all":{"spotted":378,"hits":1363,"frags":177,"max_xp":1813,"wins":118,"losses":117,"capture_points":55,"battles":235,"damage_dealt":225858,"damage_received":250855,"max_frags":4,"shots":1629,"frags8p":146,"xp":165757,"win_and_survived":55,"survived_battles":59,"dropped_capture_points":94},"last_battle_time":1621119377,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":31891,"in_garage":null,"tank_id":4929},{"all":{"spotted":34,"hits":269,"frags":36,"max_xp":945,"wins":16,"losses":25,"capture_points":1,"battles":41,"damage_dealt":25866,"damage_received":20416,"max_frags":4,"shots":341,"frags8p":0,"xp":16718,"win_and_survived":12,"survived_battles":12,"dropped_capture_points":1},"last_battle_time":1577800469,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":6516,"in_garage":null,"tank_id":1},{"all":{"spotted":3,"hits":76,"frags":8,"max_xp":569,"wins":6,"losses":2,"capture_points":2,"battles":8,"damage_dealt":4072,"damage_received":2274,"max_frags":3,"shots":102,"frags8p":0,"xp":2654,"win_and_survived":4,"survived_battles":4,"dropped_capture_points":13},"last_battle_time":1499522791,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1526,"in_garage":null,"tank_id":321},{"all":{"spotted":14,"hits":139,"frags":9,"max_xp":957,"wins":8,"losses":7,"capture_points":0,"battles":15,"damage_dealt":7182,"damage_received":6044,"max_frags":3,"shots":187,"frags8p":0,"xp":6602,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":12},"last_battle_time":1615120479,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":2306,"in_garage":null,"tank_id":52225},{"all":{"spotted":42,"hits":257,"frags":19,"max_xp":1353,"wins":19,"losses":16,"capture_points":0,"battles":35,"damage_dealt":39903,"damage_received":47232,"max_frags":2,"shots":320,"frags8p":13,"xp":25455,"win_and_survived":12,"survived_battles":13,"dropped_capture_points":10},"last_battle_time":1621356999,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":7926,"in_garage":null,"tank_id":19745},{"all":{"spotted":29,"hits":141,"frags":23,"max_xp":1581,"wins":9,"losses":10,"capture_points":0,"battles":19,"damage_dealt":18296,"damage_received":14563,"max_frags":6,"shots":171,"frags8p":0,"xp":12114,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":0},"last_battle_time":1620936807,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":2371,"in_garage":null,"tank_id":20817},{"all":{"spotted":73,"hits":432,"frags":38,"max_xp":1633,"wins":34,"losses":19,"capture_points":0,"battles":53,"damage_dealt":73473,"damage_received":53667,"max_frags":3,"shots":513,"frags8p":33,"xp":49017,"win_and_survived":25,"survived_battles":25,"dropped_capture_points":0},"last_battle_time":1616163474,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":8849,"in_garage":null,"tank_id":897},{"all":{"spotted":99,"hits":748,"frags":107,"max_xp":1219,"wins":72,"losses":73,"capture_points":0,"battles":145,"damage_dealt":106971,"damage_received":87136,"max_frags":4,"shots":955,"frags8p":4,"xp":67133,"win_and_survived":52,"survived_battles":59,"dropped_capture_points":162},"last_battle_time":1612388654,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":25778,"in_garage":null,"tank_id":11553},{"all":{"spotted":4,"hits":13,"frags":1,"max_xp":799,"wins":2,"losses":0,"capture_points":0,"battles":2,"damage_dealt":1216,"damage_received":1232,"max_frags":1,"shots":16,"frags8p":0,"xp":1396,"win_and_survived":1,"survived_battles":1,"dropped_capture_points":0},"last_battle_time":1577705474,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":256,"in_garage":null,"tank_id":9761},{"all":{"spotted":131,"hits":1871,"frags":228,"max_xp":1540,"wins":141,"losses":143,"capture_points":0,"battles":284,"damage_dealt":289134,"damage_received":198101,"max_frags":5,"shots":2429,"frags8p":11,"xp":158253,"win_and_survived":117,"survived_battles":127,"dropped_capture_points":0},"last_battle_time":1609511192,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":51830,"in_garage":null,"tank_id":11041},{"all":{"spotted":8,"hits":47,"frags":9,"max_xp":1401,"wins":4,"losses":6,"capture_points":0,"battles":10,"damage_dealt":12739,"damage_received":13102,"max_frags":4,"shots":55,"frags8p":1,"xp":6613,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1622025871,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":1387,"in_garage":null,"tank_id":59137},{"all":{"spotted":21,"hits":567,"frags":26,"max_xp":647,"wins":8,"losses":9,"capture_points":0,"battles":17,"damage_dealt":7746,"damage_received":4975,"max_frags":4,"shots":814,"frags8p":0,"xp":6277,"win_and_survived":7,"survived_battles":7,"dropped_capture_points":22},"last_battle_time":1500477618,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":2833,"in_garage":null,"tank_id":13073},{"all":{"spotted":1,"hits":32,"frags":8,"max_xp":973,"wins":6,"losses":4,"capture_points":0,"battles":10,"damage_dealt":6631,"damage_received":4556,"max_frags":3,"shots":40,"frags8p":0,"xp":4938,"win_and_survived":5,"survived_battles":5,"dropped_capture_points":0},"last_battle_time":1620568577,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1578,"in_garage":null,"tank_id":9809},{"all":{"spotted":36,"hits":1949,"frags":45,"max_xp":925,"wins":31,"losses":11,"capture_points":43,"battles":43,"damage_dealt":18735,"damage_received":11289,"max_frags":3,"shots":2802,"frags8p":0,"xp":24062,"win_and_survived":21,"survived_battles":21,"dropped_capture_points":154},"last_battle_time":1621792260,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":7312,"in_garage":null,"tank_id":54865},{"all":{"spotted":0,"hits":23,"frags":3,"max_xp":311,"wins":2,"losses":0,"capture_points":40,"battles":2,"damage_dealt":72,"damage_received":216,"max_frags":2,"shots":45,"frags8p":0,"xp":511,"win_and_survived":2,"survived_battles":2,"dropped_capture_points":0},"last_battle_time":1497565134,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":419,"in_garage":null,"tank_id":1601},{"all":{"spotted":131,"hits":354,"frags":33,"max_xp":1320,"wins":39,"losses":26,"capture_points":53,"battles":65,"damage_dealt":44939,"damage_received":52374,"max_frags":2,"shots":402,"frags8p":0,"xp":37940,"win_and_survived":17,"survived_battles":17,"dropped_capture_points":24},"last_battle_time":1621118614,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":7155,"in_garage":null,"tank_id":6465},{"all":{"spotted":3,"hits":82,"frags":15,"max_xp":634,"wins":5,"losses":5,"capture_points":3,"battles":11,"damage_dealt":7221,"damage_received":2729,"max_frags":4,"shots":98,"frags8p":0,"xp":3562,"win_and_survived":5,"survived_battles":6,"dropped_capture_points":94},"last_battle_time":1560538022,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":2823,"in_garage":null,"tank_id":1809},{"all":{"spotted":15,"hits":92,"frags":17,"max_xp":752,"wins":8,"losses":6,"capture_points":2,"battles":14,"damage_dealt":6600,"damage_received":6000,"max_frags":5,"shots":114,"frags8p":0,"xp":4745,"win_and_survived":5,"survived_battles":5,"dropped_capture_points":21},"last_battle_time":1614723599,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":3,"battle_life_time":2102,"in_garage":null,"tank_id":1537},{"all":{"spotted":8,"hits":62,"frags":8,"max_xp":840,"wins":4,"losses":4,"capture_points":93,"battles":8,"damage_dealt":4539,"damage_received":4216,"max_frags":4,"shots":81,"frags8p":0,"xp":3751,"win_and_survived":3,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1615738294,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1270,"in_garage":null,"tank_id":53585},{"all":{"spotted":4,"hits":35,"frags":6,"max_xp":314,"wins":3,"losses":3,"capture_points":0,"battles":6,"damage_dealt":1992,"damage_received":1156,"max_frags":2,"shots":49,"frags8p":0,"xp":1291,"win_and_survived":2,"survived_battles":3,"dropped_capture_points":0},"last_battle_time":1478469603,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":1,"battle_life_time":1196,"in_garage":null,"tank_id":353},{"all":{"spotted":49,"hits":372,"frags":62,"max_xp":1162,"wins":40,"losses":26,"capture_points":0,"battles":66,"damage_dealt":55766,"damage_received":45990,"max_frags":4,"shots":478,"frags8p":0,"xp":33645,"win_and_survived":28,"survived_battles":29,"dropped_capture_points":0},"last_battle_time":1620476688,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":10330,"in_garage":null,"tank_id":4993},{"all":{"spotted":20,"hits":304,"frags":43,"max_xp":1189,"wins":24,"losses":27,"capture_points":0,"battles":51,"damage_dealt":42223,"damage_received":35181,"max_frags":4,"shots":385,"frags8p":0,"xp":21607,"win_and_survived":16,"survived_battles":16,"dropped_capture_points":4},"last_battle_time":1577626951,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":2,"battle_life_time":9175,"in_garage":null,"tank_id":7201},{"all":{"spotted":9,"hits":83,"frags":13,"max_xp":744,"wins":8,"losses":4,"capture_points":5,"battles":12,"damage_dealt":6446,"damage_received":4816,"max_frags":6,"shots":92,"frags8p":0,"xp":4084,"win_and_survived":6,"survived_battles":6,"dropped_capture_points":0},"last_battle_time":1474535330,"account_id":5589968,"max_xp":0,"in_garage_updated":0,"max_frags":0,"frags":null,"mark_of_mastery":4,"battle_life_time":2555,"in_garage":null,"tank_id":64081}]}"#,
        )?;
        Ok(())
    }

    #[test]
    fn tank_achievements_ok() -> crate::Result {
        serde_json::from_str::<ResultMapVec<TankAchievements>>(
            // language=json
            r#"{"5589968":[{"achievements":{"medalCarius":4,"medalLehvaslaiho":1,"medalAbrams":4,"armorPiercer":1,"medalPoppel":4,"markOfMasteryII":6,"supporter":1,"medalKay":4,"warrior":2,"mainGun":2,"titleSniper":1,"markOfMasteryIII":4,"medalKnispel":4},"max_series":{"armorPiercer":20,"punisher":0,"titleSniper":21,"invincible":1,"tankExpert":0,"medalKay":5,"diehard":2,"beasthunter":1,"handOfDeath":2,"jointVictory":0,"sinai":0,"pattonValley":0},"account_id":5589968,"tank_id":1}]}"#,
        )?;
        Ok(())
    }

    #[test]
    fn tankopedia_ok() -> crate::Result {
        serde_json::from_str::<ResultMap<Vehicle>>(
            // language=json
            r#"{"1649":{"suspensions":[1138],"description":"Неостановимый Дракула возродился, и тьма нависла над миром. Долг зовёт охотника на вампиров Хелсинга вновь встать на защиту Света и дать бой древнему злу. Воплощение Хелсинга — это произведение искусства, инкрустированная защитными орнаментами боевая машина, снаряжённая специально для борьбы с порождениями тьмы. Сдвоенное орудие Хелсинга стреляет два раза автоматически — только так можно остановить полёт Дракулы и одержать победу.\r\nПремиум танк «Хелсинг H0» можно было получить во время игрового события «Ночная охота» в октябре 2016 года.","engines":[17013],"prices_xp":null,"next_tanks":null,"modules_tree":{"1138":{"name":"Helsing type1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1138,"type":"vehicleChassis"},"1139":{"name":"Helsing type1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1139,"type":"vehicleTurret"},"1140":{"name":"85mm Twin X-Barrel mod1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1140,"type":"vehicleGun"},"17013":{"name":"Aether W-20","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":17013,"type":"vehicleEngine"}},"nation":"other","is_premium":true,"images":{"preview":"http:\/\/glossary-ru-static.gcdn.co\/icons\/wotb\/current\/uploaded\/vehicles\/hd_thumbnail\/Van_Helsing.png","normal":"http:\/\/glossary-ru-static.gcdn.co\/icons\/wotb\/current\/uploaded\/vehicles\/hd\/Van_Helsing.png"},"cost":null,"default_profile":{"weight":24880,"profile_id":"1138-1139-1140-17013","firepower":62,"shot_efficiency":67,"gun_id":1140,"signal_range":null,"shells":[{"type":"ARMOR_PIERCING","penetration":170,"damage":200},{"type":"ARMOR_PIERCING_CR","penetration":220,"damage":170},{"type":"HIGH_EXPLOSIVE","penetration":45,"damage":300}],"armor":{"turret":{"front":80,"sides":50,"rear":40},"hull":{"front":60,"sides":40,"rear":40}},"speed_forward":60,"battle_level_range_min":7,"speed_backward":15,"engine":{"tier":8,"fire_chance":0.2,"power":500,"name":"Aether W-20","weight":530},"max_ammo":100,"battle_level_range_max":8,"engine_id":17013,"hp":1000,"is_default":true,"protection":30,"suspension":{"tier":7,"load_limit":27800,"traverse_speed":30,"name":"Helsing type1","weight":6000},"suspension_id":1138,"max_weight":27800,"gun":{"move_down_arc":6,"caliber":85,"name":"85mm Twin X-Barrel mod1","weight":3800,"move_up_arc":15,"fire_rate":11.71,"clip_reload_time":0.25,"dispersion":0.34,"clip_capacity":2,"traverse_speed":43.75,"reload_time":10.0,"tier":8,"aim_time":4.2},"turret_id":1139,"turret":{"name":"Helsing type1","weight":3350,"view_range":240,"traverse_left_arc":180,"hp":200,"traverse_speed":17,"tier":7,"traverse_right_arc":180},"maneuverability":53,"hull_weight":10950,"hull_hp":800},"tier":7,"tank_id":1649,"type":"AT-SPG","guns":[1140],"turrets":[1139],"name":"Helsing"}}"#,
        )?;
        Ok(())
    }
}
