use std::ops::AddAssign;

use bpci::{Interval, NSuccessesSample, WilsonScore};

use crate::database;
use crate::prelude::*;

#[derive(Copy, Clone, Default)]
pub struct Sample {
    pub n_battles: u32,
    pub n_wins: u32,
}

impl From<&database::TrainItem> for Sample {
    fn from(item: &database::TrainItem) -> Self {
        Self {
            n_battles: item.n_battles,
            n_wins: item.n_wins,
        }
    }
}

impl AddAssign<&Self> for Sample {
    fn add_assign(&mut self, rhs: &Self) {
        self.n_battles += rhs.n_battles;
        self.n_wins += rhs.n_wins;
    }
}

impl Sample {
    pub fn victory_ratio(self, z_level: f64) -> Result<f64> {
        Ok(NSuccessesSample::new(self.n_battles, self.n_wins)?
            .wilson_score_with_cc(z_level)
            .mean())
    }
}
