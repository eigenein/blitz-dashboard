use std::cmp::Ordering;
use std::ops::{Add, Mul};

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
#[must_use]
pub struct ConfidenceInterval {
    pub mean: f64,
    pub margin: f64,
}

impl ConfidenceInterval {
    /// <https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval>
    pub fn wilson_score_interval(n_trials: i32, n_successes: i32, level: ConfidenceLevel) -> Self {
        let z = level.z_value();
        let n_trials = n_trials as f64;
        let n_successes = n_successes as f64;

        let p_hat = n_successes / n_trials;

        let a = z * z / n_trials;
        let b = 1.0 / (1.0 + a);

        let mean = b * (p_hat + a / 2.0);
        let margin = z * b * (p_hat * (1.0 - p_hat) / n_trials + a / n_trials / 4.0).sqrt();

        Self { mean, margin }
    }

    #[must_use]
    pub fn lower(&self) -> f64 {
        self.mean - self.margin
    }

    #[must_use]
    pub fn upper(&self) -> f64 {
        self.mean + self.margin
    }
}

impl Mul<f64> for ConfidenceInterval {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::Output {
            mean: self.mean * rhs,
            margin: self.margin * rhs,
        }
    }
}

impl Mul<ConfidenceInterval> for f64 {
    type Output = ConfidenceInterval;

    fn mul(self, rhs: ConfidenceInterval) -> Self::Output {
        rhs * self
    }
}

impl Add<f64> for ConfidenceInterval {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self::Output {
            mean: self.mean + rhs,
            margin: self.margin,
        }
    }
}

impl Add<ConfidenceInterval> for f64 {
    type Output = ConfidenceInterval;

    fn add(self, rhs: ConfidenceInterval) -> Self::Output {
        rhs + self
    }
}

impl PartialEq for ConfidenceInterval {
    #[must_use]
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl PartialOrd for ConfidenceInterval {
    #[must_use]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.upper() < other.lower() {
            Some(Ordering::Less)
        } else if self.lower() > other.upper() {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone)]
pub enum ConfidenceLevel {
    Z80,
    Z85,
    Z87,
    Z88,
    Z89,
    Z90,
    Z95,
    Z96,
    Z97,
    Z98,
    Z99,
    Z99_99,
}

impl Default for ConfidenceLevel {
    fn default() -> Self {
        Self::Z96
    }
}

impl ConfidenceLevel {
    pub const fn z_value(&self) -> f64 {
        match self {
            Self::Z80 => 1.28,
            Self::Z85 => 1.440,
            Self::Z87 => 1.51,
            Self::Z88 => 1.5548,
            Self::Z89 => 1.598,
            Self::Z90 => 1.645,
            Self::Z95 => 1.960,
            Self::Z96 => 2.054,
            Self::Z97 => 2.17009,
            Self::Z98 => 2.326,
            Self::Z99 => 2.576,
            Self::Z99_99 => 3.29053,
        }
    }
}
