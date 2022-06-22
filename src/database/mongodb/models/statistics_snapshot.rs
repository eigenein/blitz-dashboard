use serde::{Deserialize, Serialize};

use crate::math::statistics::{ConfidenceInterval, ConfidenceLevel};
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct StatisticsSnapshot {
    #[serde(rename = "life")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub battle_life_time: Duration,

    #[serde(rename = "nb")]
    pub n_battles: i32,

    #[serde(rename = "nw")]
    pub n_wins: i32,

    #[serde(rename = "nsb")]
    pub n_survived_battles: i32,

    #[serde(rename = "nws")]
    pub n_win_and_survived: i32,

    #[serde(rename = "dmgd")]
    pub damage_dealt: i32,

    #[serde(rename = "dmgr")]
    pub damage_received: i32,

    #[serde(rename = "shts")]
    pub n_shots: i32,

    #[serde(rename = "hits")]
    pub n_hits: i32,

    #[serde(rename = "frgs")]
    pub n_frags: i32,

    #[serde(rename = "xp")]
    pub xp: i32,
}

impl From<wargaming::TankStatistics> for StatisticsSnapshot {
    fn from(statistics: wargaming::TankStatistics) -> Self {
        Self {
            battle_life_time: statistics.battle_life_time,
            n_battles: statistics.all.battles,
            n_wins: statistics.all.wins,
            n_survived_battles: statistics.all.survived_battles,
            n_win_and_survived: statistics.all.win_and_survived,
            damage_dealt: statistics.all.damage_dealt,
            damage_received: statistics.all.damage_received,
            n_shots: statistics.all.shots,
            n_hits: statistics.all.hits,
            n_frags: statistics.all.frags,
            xp: statistics.all.xp,
        }
    }
}

impl StatisticsSnapshot {
    #[must_use]
    #[inline]
    pub fn current_win_rate(&self) -> f64 {
        self.n_wins as f64 / self.n_battles as f64
    }

    #[inline]
    pub fn true_win_rate(&self) -> ConfidenceInterval {
        ConfidenceInterval::wilson_score_interval(
            self.n_battles,
            self.n_wins,
            ConfidenceLevel::default(),
        )
    }

    #[must_use]
    #[inline]
    pub fn frags_per_battle(&self) -> f64 {
        self.n_frags as f64 / self.n_battles as f64
    }

    #[must_use]
    #[inline]
    pub fn wins_per_hour(&self) -> f64 {
        self.n_wins as f64 / self.battle_life_time.num_seconds() as f64 * 3600.0
    }

    #[must_use]
    #[inline]
    pub fn battles_per_hour(&self) -> f64 {
        self.n_battles as f64 / self.battle_life_time.num_seconds() as f64 * 3600.0
    }

    #[must_use]
    #[inline]
    pub fn damage_per_minute(&self) -> f64 {
        self.damage_dealt as f64 / self.battle_life_time.num_seconds() as f64 * 60.0
    }
}
