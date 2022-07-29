use bpci::{BoundedInterval, NSuccessesSample, WilsonScore};

use crate::math::statistics::ConfidenceLevel;
use crate::Result;

pub trait NWins {
    fn n_wins(&self) -> u32;
}

pub trait NBattles {
    fn n_battles(&self) -> u32;
}

pub trait NSurvivedBattles {
    fn n_survived_battles(&self) -> u32;
}

pub trait DamageDealt {
    fn damage_dealt(&self) -> u32;
}

pub trait TrueWinRate {
    fn true_win_rate(&self) -> Result<BoundedInterval<f64>>;
}

impl<T: NBattles + NWins> TrueWinRate for T {
    fn true_win_rate(&self) -> Result<BoundedInterval<f64>> {
        let sample = NSuccessesSample::new(self.n_battles(), self.n_wins())?;
        Ok(sample.wilson_score_with_cc(ConfidenceLevel::default().z_value()))
    }
}

pub trait TrueSurvivalRate {
    fn true_survival_rate(&self) -> Result<BoundedInterval<f64>>;
}

impl<T: NBattles + NSurvivedBattles> TrueSurvivalRate for T {
    fn true_survival_rate(&self) -> Result<BoundedInterval<f64>> {
        let sample = NSuccessesSample::new(self.n_battles(), self.n_survived_battles())?;
        Ok(sample.wilson_score_with_cc(ConfidenceLevel::default().z_value()))
    }
}

pub trait CurrentWinRate {
    fn current_win_rate(&self) -> f64;
}

impl<T: NBattles + NWins> CurrentWinRate for T {
    fn current_win_rate(&self) -> f64 {
        self.n_wins() as f64 / self.n_battles() as f64
    }
}

pub trait AverageDamageDealt {
    fn average_damage_dealt(&self) -> f64;
}

impl<T: NBattles + DamageDealt> AverageDamageDealt for T {
    fn average_damage_dealt(&self) -> f64 {
        self.damage_dealt() as f64 / self.n_battles() as f64
    }
}
