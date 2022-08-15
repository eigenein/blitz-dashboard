use std::ops::Sub;

use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;

use crate::math::traits::{DamageDealt, NBattles, NWins};
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct RatingStatsSnapshot {
    #[serde(rename = "mm")]
    pub mm_rating: wargaming::MmRating,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nrb")]
    pub n_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nrw")]
    pub n_wins: u32,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "rdmgd")]
    pub damage_dealt: u64,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "rdmgr")]
    pub damage_received: u64,

    #[serde(default, rename = "szn")]
    pub current_season: u16,
}

impl NBattles for RatingStatsSnapshot {
    fn n_battles(&self) -> u32 {
        self.n_battles
    }
}

impl NWins for RatingStatsSnapshot {
    fn n_wins(&self) -> u32 {
        self.n_wins
    }
}

impl DamageDealt for RatingStatsSnapshot {
    fn damage_dealt(&self) -> u64 {
        self.damage_dealt
    }
}

impl From<wargaming::RatingStats> for RatingStatsSnapshot {
    fn from(stats: wargaming::RatingStats) -> Self {
        Self {
            mm_rating: stats.mm_rating,
            n_battles: stats.basic.n_battles,
            n_wins: stats.basic.n_wins,
            damage_dealt: stats.basic.damage_dealt,
            damage_received: stats.basic.damage_received,
            current_season: stats.current_season,
        }
    }
}

impl Sub<RatingStatsSnapshot> for wargaming::RatingStats {
    type Output = RatingStatsSnapshot;

    fn sub(self, rhs: RatingStatsSnapshot) -> Self::Output {
        Self::Output {
            mm_rating: (self.mm_rating.0 - rhs.mm_rating.0).into(),
            n_battles: self.basic.n_battles - rhs.n_battles,
            n_wins: self.basic.n_wins - rhs.n_wins,
            damage_dealt: self.basic.damage_dealt - rhs.damage_dealt,
            damage_received: self.basic.damage_received - rhs.damage_received,
            current_season: self.current_season,
        }
    }
}

impl RatingStatsSnapshot {
    #[must_use]
    pub fn delta(&self) -> f64 {
        self.mm_rating.0 * 10.0
    }

    #[must_use]
    pub fn delta_per_battle(&self) -> f64 {
        self.delta() / self.n_battles as f64
    }
}
