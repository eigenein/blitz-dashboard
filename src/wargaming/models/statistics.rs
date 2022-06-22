use serde::{Deserialize, Serialize};

use crate::database;

#[must_use]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct BasicStatistics {
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

impl From<&database::StatisticsSnapshot> for BasicStatistics {
    fn from(snapshot: &database::StatisticsSnapshot) -> Self {
        Self {
            battles: snapshot.n_battles,
            wins: snapshot.n_wins,
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct RatingStatistics {
    #[serde(flatten)]
    pub basic: BasicStatistics,

    #[serde(default)]
    pub mm_rating: Option<f64>,
}

impl RatingStatistics {
    #[allow(dead_code)]
    pub fn client_rating(&self) -> Option<f64> {
        self.mm_rating.map(|mm_rating| mm_rating * 10.0 + 3000.0)
    }
}
