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
    fn damage_dealt(&self) -> u64;
}

pub trait TrueWinRate {
    fn true_win_rate(&self, confidence_level: ConfidenceLevel) -> Result<BoundedInterval<f64>>;

    fn posterior_victory_probability(&self) -> f64;
}

impl<T: NBattles + NWins> TrueWinRate for T {
    fn true_win_rate(&self, confidence_level: ConfidenceLevel) -> Result<BoundedInterval<f64>> {
        let sample = NSuccessesSample::new(self.n_battles(), self.n_wins())?;
        Ok(sample.wilson_score_with_cc(confidence_level.z_value()))
    }

    fn posterior_victory_probability(&self) -> f64 {
        (self.n_wins() + 2) as f64 / (self.n_battles() + 4) as f64
    }
}

pub trait TrueSurvivalRate {
    fn true_survival_rate(&self, confidence_level: ConfidenceLevel)
    -> Result<BoundedInterval<f64>>;
}

impl<T: NBattles + NSurvivedBattles> TrueSurvivalRate for T {
    fn true_survival_rate(
        &self,
        confidence_level: ConfidenceLevel,
    ) -> Result<BoundedInterval<f64>> {
        let sample = NSuccessesSample::new(self.n_battles(), self.n_survived_battles())?;
        Ok(sample.wilson_score_with_cc(confidence_level.z_value()))
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

pub trait DamageReceived {
    fn damage_received(&self) -> u64;
}

pub trait DamageRatio {
    fn damage_ratio(&self) -> f64;
}

impl<T: DamageDealt + DamageReceived> DamageRatio for T {
    fn damage_ratio(&self) -> f64 {
        self.damage_dealt() as f64 / self.damage_received() as f64
    }
}
