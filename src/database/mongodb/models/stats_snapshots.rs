use std::iter::Sum;
use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::math::statistics::{ConfidenceInterval, ConfidenceLevel};
use crate::wargaming;

/// This is a part of the other models, there's no dedicated collection
/// for statistics snapshots.
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone, Default)]
pub struct RandomStatsSnapshot {
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

impl From<wargaming::BasicStatistics> for RandomStatsSnapshot {
    fn from(statistics: wargaming::BasicStatistics) -> Self {
        Self {
            n_battles: statistics.n_battles,
            n_wins: statistics.n_wins,
            n_survived_battles: statistics.survived_battles,
            n_win_and_survived: statistics.win_and_survived,
            damage_dealt: statistics.damage_dealt,
            damage_received: statistics.damage_received,
            n_shots: statistics.shots,
            n_hits: statistics.hits,
            n_frags: statistics.frags,
            xp: statistics.xp,
        }
    }
}

impl Sum for RandomStatsSnapshot {
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

impl RandomStatsSnapshot {
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

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct RatingStatsSnapshot {
    #[serde(rename = "mm")]
    pub mm_rating: f64,

    #[serde(default, rename = "nrb")]
    pub n_battles: i32,

    #[serde(default, rename = "nrw")]
    pub n_wins: i32,
}

impl From<wargaming::RatingStatistics> for RatingStatsSnapshot {
    fn from(stats: wargaming::RatingStatistics) -> Self {
        Self {
            mm_rating: stats.mm_rating,
            n_battles: stats.basic.n_battles,
            n_wins: stats.basic.n_wins,
        }
    }
}

impl Sub<RatingStatsSnapshot> for wargaming::RatingStatistics {
    type Output = RatingStatsSnapshot;

    fn sub(self, rhs: RatingStatsSnapshot) -> Self::Output {
        Self::Output {
            mm_rating: self.mm_rating - rhs.mm_rating,
            n_battles: self.basic.n_battles - rhs.n_battles,
            n_wins: self.basic.n_wins - rhs.n_wins,
        }
    }
}

impl RatingStatsSnapshot {
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
    pub fn current_win_rate(&self) -> f64 {
        self.n_wins as f64 / self.n_battles as f64
    }

    #[must_use]
    pub fn delta(&self) -> f64 {
        self.mm_rating * 10.0
    }
}
