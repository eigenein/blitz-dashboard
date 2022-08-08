use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::database;
use crate::math::traits::{DamageDealt, NBattles, NWins};
use crate::wargaming::MmRating;

#[must_use]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct BasicStats {
    #[serde(rename = "battles")]
    pub n_battles: u32,

    #[serde(rename = "wins")]
    pub n_wins: u32,

    pub survived_battles: u32,
    pub win_and_survived: u32,
    pub damage_dealt: u32,
    pub damage_received: u32,
    pub shots: u32,
    pub hits: u32,
    pub frags: u32,
    pub xp: u32,
}

impl From<&database::RandomStatsSnapshot> for BasicStats {
    fn from(snapshot: &database::RandomStatsSnapshot) -> Self {
        Self {
            n_battles: snapshot.n_battles,
            n_wins: snapshot.n_wins,
            survived_battles: snapshot.n_survived_battles,
            win_and_survived: snapshot.n_win_and_survived,
            damage_dealt: snapshot.damage_dealt,
            damage_received: snapshot.damage_received,
            shots: snapshot.n_shots,
            hits: snapshot.n_hits,
            frags: snapshot.n_frags,
            xp: snapshot.xp,
        }
    }
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

impl NBattles for BasicStats {
    fn n_battles(&self) -> u32 {
        self.n_battles
    }
}

impl NWins for BasicStats {
    fn n_wins(&self) -> u32 {
        self.n_wins
    }
}

impl DamageDealt for BasicStats {
    fn damage_dealt(&self) -> u32 {
        self.damage_dealt
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct RatingStats {
    #[serde(flatten)]
    pub basic: BasicStats,

    #[serde(default)]
    pub mm_rating: MmRating,

    pub current_season: u16,
}
