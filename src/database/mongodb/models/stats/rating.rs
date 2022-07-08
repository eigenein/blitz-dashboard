use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::math::statistics::{ConfidenceInterval, ConfidenceLevel};
use crate::wargaming;

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

    #[must_use]
    pub fn delta_per_battle(&self) -> f64 {
        self.delta() / self.n_battles as f64
    }
}
