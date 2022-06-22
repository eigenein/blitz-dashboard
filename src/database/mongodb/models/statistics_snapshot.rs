use std::iter::Sum;

use serde::{Deserialize, Serialize};

use crate::math::statistics::{ConfidenceInterval, ConfidenceLevel};
use crate::prelude::*;
use crate::wargaming;

/// This is a part of the other models, there's no dedicated collection
/// for statistics snapshots.
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

impl Default for StatisticsSnapshot {
    fn default() -> Self {
        Self {
            battle_life_time: Duration::seconds(0),
            n_battles: 0,
            n_wins: 0,
            n_survived_battles: 0,
            n_win_and_survived: 0,
            damage_dealt: 0,
            damage_received: 0,
            n_shots: 0,
            n_hits: 0,
            n_frags: 0,
            xp: 0,
        }
    }
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

impl Sum for StatisticsSnapshot {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum = Self::default();
        for component in iter {
            sum.n_battles += component.n_battles;
            sum.n_wins += component.n_wins;
            sum.n_hits += component.n_hits;
            sum.n_shots += component.n_shots;
            sum.n_survived_battles += component.n_survived_battles;
            sum.n_frags += component.n_frags;
            sum.xp += component.xp;
            sum.damage_received += component.damage_received;
            sum.damage_dealt += component.damage_dealt;
            sum.n_win_and_survived += component.n_win_and_survived;
        }
        sum
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

    #[must_use]
    #[inline]
    pub fn damage_per_battle(&self) -> f64 {
        self.damage_dealt as f64 / self.n_battles as f64
    }

    #[must_use]
    #[inline]
    pub fn survival_rate(&self) -> f64 {
        self.n_survived_battles as f64 / self.n_battles as f64
    }

    #[must_use]
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        self.n_hits as f64 / self.n_shots as f64
    }
}
