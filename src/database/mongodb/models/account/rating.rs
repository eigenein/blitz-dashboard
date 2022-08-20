use std::ops::Sub;

use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;

use crate::helpers::serde::is_default;
use crate::math::traits::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct RatingStatsSnapshot {
    #[serde(default, rename = "mm", skip_serializing_if = "is_default")]
    pub mm_rating: wargaming::MmRating,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nrb", skip_serializing_if = "is_default")]
    pub n_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nrw", skip_serializing_if = "is_default")]
    pub n_wins: u32,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "rdmgd", skip_serializing_if = "is_default")]
    pub damage_dealt: u64,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "rdmgr", skip_serializing_if = "is_default")]
    pub damage_received: u64,

    #[serde(default, rename = "szn", skip_serializing_if = "is_default")]
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

impl DamageReceived for RatingStatsSnapshot {
    fn damage_received(&self) -> u64 {
        self.damage_received
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
            n_battles: self.basic.n_battles.saturating_sub(rhs.n_battles),
            n_wins: self.basic.n_wins.saturating_sub(rhs.n_wins),
            damage_dealt: self.basic.damage_dealt.saturating_sub(rhs.damage_dealt),
            damage_received: self
                .basic
                .damage_received
                .saturating_sub(rhs.damage_received),
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
