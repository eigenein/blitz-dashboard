use serde::{Deserialize, Serialize};

use crate::database;
use crate::math::traits::{DamageDealt, NBattles, NWins};

#[must_use]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct BasicStats {
    #[serde(rename = "battles")]
    pub n_battles: i32,

    #[serde(rename = "wins")]
    pub n_wins: i32,

    pub survived_battles: i32,
    pub win_and_survived: i32,
    pub damage_dealt: i32,
    pub damage_received: i32,
    pub shots: i32,
    pub hits: i32,
    pub frags: i32,
    pub xp: i32,
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

impl NBattles for BasicStats {
    fn n_battles(&self) -> i32 {
        self.n_battles
    }
}

impl NWins for BasicStats {
    fn n_wins(&self) -> i32 {
        self.n_wins
    }
}

impl DamageDealt for BasicStats {
    fn damage_dealt(&self) -> i32 {
        self.damage_dealt
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct RatingStats {
    #[serde(flatten)]
    pub basic: BasicStats,

    #[serde(default)]
    pub mm_rating: f64,
}

impl RatingStats {
    #[must_use]
    pub fn rating(&self) -> f64 {
        self.mm_rating * 10.0 + 3000.0
    }
}
