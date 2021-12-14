use std::cmp::Ordering;
use std::ops::{Add, Mul};

use crate::Float;

#[allow(dead_code)]
#[must_use]
pub fn mean(values: &[Float]) -> Float {
    values.iter().sum::<Float>() / values.len().max(1) as Float
}

#[derive(Copy, Clone)]
pub struct ConfidenceInterval {
    pub mean: Float,
    pub margin: Float,
}

impl ConfidenceInterval {
    /// <https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval>
    #[must_use]
    pub fn wilson_score_interval(n_trials: i32, n_successes: i32, z: Float) -> Self {
        let n_trials = n_trials as Float;
        let n_successes = n_successes as Float;

        let p_hat = n_successes / n_trials;

        let a = z * z / n_trials;
        let b = 1.0 / (1.0 + a);

        let mean = b * (p_hat + a / 2.0);
        let margin = z * b * (p_hat * (1.0 - p_hat) / n_trials + a / n_trials / 4.0).sqrt();

        Self { mean, margin }
    }

    #[must_use]
    pub fn default_wilson_score_interval(n_trials: i32, n_successes: i32) -> Self {
        Self::wilson_score_interval(n_trials, n_successes, Z_95)
    }

    #[must_use]
    pub fn lower(&self) -> Float {
        self.mean - self.margin
    }

    #[must_use]
    pub fn upper(&self) -> Float {
        self.mean + self.margin
    }
}

impl Mul<Float> for ConfidenceInterval {
    type Output = Self;

    #[must_use]
    fn mul(self, rhs: Float) -> Self::Output {
        Self::Output {
            mean: self.mean * rhs,
            margin: self.margin * rhs,
        }
    }
}

impl Mul<ConfidenceInterval> for Float {
    type Output = ConfidenceInterval;

    #[must_use]
    fn mul(self, rhs: ConfidenceInterval) -> Self::Output {
        rhs * self
    }
}

impl Add<Float> for ConfidenceInterval {
    type Output = Self;

    #[must_use]
    fn add(self, rhs: Float) -> Self::Output {
        Self::Output {
            mean: self.mean + rhs,
            margin: self.margin,
        }
    }
}

impl Add<ConfidenceInterval> for Float {
    type Output = ConfidenceInterval;

    #[must_use]
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

#[allow(dead_code)]
pub const Z_95: Float = 1.96;

#[allow(dead_code)]
pub const Z_90: Float = 1.645;

#[allow(dead_code)]
pub const Z_89: Float = 1.598;

#[allow(dead_code)]
pub const Z_88: Float = 1.5548;

#[allow(dead_code)]
pub const Z_87: Float = 1.51;

#[allow(dead_code)]
pub const Z_85: Float = 1.44;

#[allow(dead_code)]
pub const Z_80: Float = 1.28;
