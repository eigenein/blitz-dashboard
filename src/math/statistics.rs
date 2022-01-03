use std::cmp::Ordering;
use std::ops::{Add, Mul};

#[allow(dead_code)]
#[must_use]
pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

#[derive(Copy, Clone)]
#[must_use]
pub struct ConfidenceInterval {
    pub mean: f64,
    pub margin: f64,
}

impl ConfidenceInterval {
    /// <https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval>
    pub fn wilson_score_interval(n_trials: i32, n_successes: i32, z: f64) -> Self {
        let n_trials = n_trials as f64;
        let n_successes = n_successes as f64;

        let p_hat = n_successes / n_trials;

        let a = z * z / n_trials;
        let b = 1.0 / (1.0 + a);

        let mean = b * (p_hat + a / 2.0);
        let margin = z * b * (p_hat * (1.0 - p_hat) / n_trials + a / n_trials / 4.0).sqrt();

        Self { mean, margin }
    }

    pub fn default_wilson_score_interval(n_trials: i32, n_successes: i32) -> Self {
        Self::wilson_score_interval(n_trials, n_successes, Z_95)
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

#[allow(dead_code)]
pub const Z_95: f64 = 1.96;

#[allow(dead_code)]
pub const Z_90: f64 = 1.645;

#[allow(dead_code)]
pub const Z_89: f64 = 1.598;

#[allow(dead_code)]
pub const Z_88: f64 = 1.5548;

#[allow(dead_code)]
pub const Z_87: f64 = 1.51;

#[allow(dead_code)]
pub const Z_85: f64 = 1.44;

#[allow(dead_code)]
pub const Z_80: f64 = 1.28;
