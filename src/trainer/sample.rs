use std::iter::Sum;
use std::ops::AddAssign;

use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct Sample {
    pub n_battles: u32,
    pub n_wins: u32,
}

impl From<&database::TrainItem> for Sample {
    fn from(item: &database::TrainItem) -> Self {
        Self {
            n_battles: item.n_battles as u32,
            n_wins: item.n_wins as u32,
        }
    }
}

impl AddAssign<&Self> for Sample {
    fn add_assign(&mut self, rhs: &Self) {
        self.n_battles += rhs.n_battles;
        self.n_wins += rhs.n_wins;
    }
}

impl<'a> Sum<&'a Self> for Sample {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        let mut sum = Self::default();
        for sample in iter {
            sum += sample;
        }
        sum
    }
}

impl Sample {
    const PRIOR_ALPHA: u32 = 1;
    const PRIOR_BETA: u32 = 1;

    pub fn posterior_mean(self) -> f64 {
        self.n_posterior_wins_f64() / self.n_posterior_battles_f64()
    }

    pub const fn n_posterior_wins(self) -> u32 {
        self.n_wins + Self::PRIOR_ALPHA
    }

    pub const fn n_posterior_wins_f64(self) -> f64 {
        self.n_posterior_wins() as f64
    }

    pub const fn n_posterior_battles(self) -> u32 {
        self.n_battles + Self::PRIOR_ALPHA + Self::PRIOR_BETA
    }

    pub const fn n_posterior_battles_f64(self) -> f64 {
        self.n_posterior_battles() as f64
    }
}
