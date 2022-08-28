use std::iter::Sum;
use std::ops::AddAssign;

use bpci::{Interval, NSuccessesSample, WilsonScore};

use crate::prelude::*;
use crate::trainer::train_item::CompressedTrainItem;

#[derive(Copy, Clone, Default)]
pub struct Sample {
    pub n_battles: u32,
    pub n_wins: u32,
}

impl From<&CompressedTrainItem> for Sample {
    fn from(item: &CompressedTrainItem) -> Self {
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
    pub fn victory_ratio(self, z_level: f64) -> Result<f64> {
        Ok(NSuccessesSample::new(self.n_battles, self.n_wins)?
            .wilson_score_with_cc(z_level)
            .mean())
    }
}
