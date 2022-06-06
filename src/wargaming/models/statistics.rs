use std::iter::Sum;

use serde::{Deserialize, Serialize};

use crate::math::statistics::{ConfidenceInterval, Z};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct Statistics {
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

impl Statistics {
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
        ConfidenceInterval::wilson_score_interval(self.battles, self.wins, Z::default())
    }
}

impl Sum for Statistics {
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
