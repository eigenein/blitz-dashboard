use std::iter::Sum;
use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::database;
use crate::math::statistics::{ConfidenceInterval, ConfidenceLevel};

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

impl From<&database::TankSnapshot> for BasicStatistics {
    fn from(snapshot: &database::TankSnapshot) -> Self {
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

impl BasicStatistics {
    pub fn damage_per_battle(&self) -> f64 {
        self.damage_dealt as f64 / self.battles as f64
    }

    pub fn current_win_rate(&self) -> f64 {
        self.wins as f64 / self.battles as f64
    }

    pub fn survival_rate(&self) -> f64 {
        self.survived_battles as f64 / self.battles as f64
    }

    pub fn hit_rate(&self) -> f64 {
        self.hits as f64 / self.shots as f64
    }

    pub fn frags_per_battle(&self) -> f64 {
        self.frags as f64 / self.battles as f64
    }

    pub fn true_win_rate(&self) -> ConfidenceInterval {
        ConfidenceInterval::wilson_score_interval(
            self.battles,
            self.wins,
            ConfidenceLevel::default(),
        )
    }
}

impl Sum for BasicStatistics {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum = Self::default();
        for component in iter {
            sum.battles += component.battles;
            sum.wins += component.wins;
            sum.hits += component.hits;
            sum.shots += component.shots;
            sum.survived_battles += component.survived_battles;
            sum.frags += component.frags;
            sum.xp += component.xp;
            sum.damage_received += component.damage_received;
            sum.damage_dealt += component.damage_dealt;
            sum.win_and_survived += component.win_and_survived;
        }
        sum
    }
}

impl Sub for BasicStatistics {
    type Output = BasicStatistics;

    #[must_use]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            battles: self.battles - rhs.battles,
            wins: self.wins - rhs.wins,
            survived_battles: self.survived_battles - rhs.survived_battles,
            win_and_survived: self.win_and_survived - rhs.win_and_survived,
            damage_dealt: self.damage_dealt - rhs.damage_dealt,
            damage_received: self.damage_received - rhs.damage_received,
            shots: self.shots - rhs.shots,
            hits: self.hits - rhs.hits,
            frags: self.frags - rhs.frags,
            xp: self.xp - rhs.xp,
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
