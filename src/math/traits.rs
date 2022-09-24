use bpci::{BoundedInterval, NSuccessesSample, WilsonScore};
use statrs::distribution::Beta;

use crate::math::statistics::ConfidenceLevel;
use crate::Result;

const PRIOR_ALPHA: u32 = 1;

pub trait NWins {
    fn n_wins(&self) -> u32;

    fn n_posterior_wins(&self) -> f64 {
        (self.n_wins() + PRIOR_ALPHA) as f64
    }
}

const PRIOR_BETA: u32 = 1;

pub trait NBattles {
    fn n_battles(&self) -> u32;

    fn n_posterior_battles(&self) -> f64 {
        (self.n_battles() + PRIOR_ALPHA + PRIOR_BETA) as f64
    }
}

pub trait NSurvivedBattles {
    fn n_survived_battles(&self) -> u32;
}

pub trait DamageDealt {
    fn damage_dealt(&self) -> u64;
}

pub trait SurvivalRatioInterval {
    fn survival_ratio_interval(
        &self,
        confidence_level: ConfidenceLevel,
    ) -> Result<BoundedInterval<f64>>;
}

impl<T: NBattles + NSurvivedBattles> SurvivalRatioInterval for T {
    fn survival_ratio_interval(
        &self,
        confidence_level: ConfidenceLevel,
    ) -> Result<BoundedInterval<f64>> {
        let sample = NSuccessesSample::new(self.n_battles(), self.n_survived_battles())?;
        Ok(sample.wilson_score_with_cc(confidence_level.z_value()))
    }
}

pub trait VictoryRatio {
    fn victory_ratio(&self) -> f64;

    fn posterior_victory_probability(&self) -> f64;

    fn victory_ratio_interval(
        &self,
        confidence_level: ConfidenceLevel,
    ) -> Result<BoundedInterval<f64>>;

    fn victory_ratio_beta(&self) -> Result<Beta>;
}

impl<T: NBattles + NWins> VictoryRatio for T {
    fn victory_ratio(&self) -> f64 {
        self.n_wins() as f64 / self.n_battles() as f64
    }

    fn posterior_victory_probability(&self) -> f64 {
        self.n_posterior_wins() / self.n_posterior_battles()
    }

    fn victory_ratio_interval(
        &self,
        confidence_level: ConfidenceLevel,
    ) -> Result<BoundedInterval<f64>> {
        let sample = NSuccessesSample::new(self.n_battles(), self.n_wins())?;
        Ok(sample.wilson_score_with_cc(confidence_level.z_value()))
    }

    fn victory_ratio_beta(&self) -> Result<Beta> {
        Ok(Beta::new(
            self.n_posterior_wins(),
            self.n_posterior_battles() - self.n_posterior_wins(),
        )?)
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
