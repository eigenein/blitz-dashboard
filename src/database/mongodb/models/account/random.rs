use std::iter::Sum;
use std::ops::Sub;

use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;

use crate::helpers::serde::is_default;
use crate::math::traits::*;
use crate::wargaming;

/// This is a part of the other models, there's no dedicated collection
/// for statistics snapshots.
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone, Default)]
pub struct RandomStatsSnapshot {
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nb", skip_serializing_if = "is_default")]
    pub n_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nw", skip_serializing_if = "is_default")]
    pub n_wins: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nsb", skip_serializing_if = "is_default")]
    pub n_survived_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nws", skip_serializing_if = "is_default")]
    pub n_win_and_survived: u32,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "dmgd", skip_serializing_if = "is_default")]
    pub damage_dealt: u64,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "dmgr", skip_serializing_if = "is_default")]
    pub damage_received: u64,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "shts", skip_serializing_if = "is_default")]
    pub n_shots: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "hits", skip_serializing_if = "is_default")]
    pub n_hits: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "frgs", skip_serializing_if = "is_default")]
    pub n_frags: u32,

    #[serde_as(as = "TryFromInto<i64>")]
    #[serde(default, rename = "xp", skip_serializing_if = "is_default")]
    pub xp: u64,
}

impl NBattles for RandomStatsSnapshot {
    fn n_battles(&self) -> u32 {
        self.n_battles
    }
}

impl NWins for RandomStatsSnapshot {
    fn n_wins(&self) -> u32 {
        self.n_wins
    }
}

impl NSurvivedBattles for RandomStatsSnapshot {
    fn n_survived_battles(&self) -> u32 {
        self.n_survived_battles
    }
}

impl DamageDealt for RandomStatsSnapshot {
    fn damage_dealt(&self) -> u64 {
        self.damage_dealt
    }
}

impl DamageReceived for RandomStatsSnapshot {
    fn damage_received(&self) -> u64 {
        self.damage_received
    }
}

impl From<wargaming::BasicStats> for RandomStatsSnapshot {
    fn from(statistics: wargaming::BasicStats) -> Self {
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

impl Sub<RandomStatsSnapshot> for RandomStatsSnapshot {
    type Output = Self;

    fn sub(self, rhs: RandomStatsSnapshot) -> Self::Output {
        Self {
            n_battles: self.n_battles.saturating_sub(rhs.n_battles),
            n_wins: self.n_wins.saturating_sub(rhs.n_wins),
            n_survived_battles: self
                .n_survived_battles
                .saturating_sub(rhs.n_survived_battles),
            n_win_and_survived: self
                .n_win_and_survived
                .saturating_sub(rhs.n_win_and_survived),
            damage_dealt: self.damage_dealt.saturating_sub(rhs.damage_dealt),
            damage_received: self.damage_received.saturating_sub(rhs.damage_received),
            n_shots: self.n_shots.saturating_sub(rhs.n_shots),
            n_hits: self.n_hits.saturating_sub(rhs.n_hits),
            n_frags: self.n_frags.saturating_sub(rhs.n_frags),
            xp: self.xp.saturating_sub(rhs.xp),
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
    pub fn frags_per_battle(&self) -> f64 {
        self.n_frags as f64 / self.n_battles as f64
    }

    #[must_use]
    #[inline]
    pub fn survival_rate(&self) -> f64 {
        self.n_survived_battles as f64 / self.n_battles as f64
    }

    #[must_use]
    #[inline]
    pub fn accuracy(&self) -> f64 {
        self.n_hits as f64 / self.n_shots as f64
    }
}
