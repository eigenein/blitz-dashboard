use std::cmp::Ordering;
use std::ops::{Add, Mul};

#[derive(Copy, Clone)]
pub struct ConfidenceInterval {
    pub mean: f64,
    pub margin: f64,
}

impl ConfidenceInterval {
    /// <https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval>
    #[must_use]
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

    #[must_use]
    pub fn default_wilson_score_interval(n_trials: i32, n_successes: i32) -> Self {
        Self::wilson_score_interval(n_trials, n_successes, Z_90)
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

    #[must_use]
    fn mul(self, rhs: f64) -> Self::Output {
        Self::Output {
            mean: self.mean * rhs,
            margin: self.margin * rhs,
        }
    }
}

impl Mul<ConfidenceInterval> for f64 {
    type Output = ConfidenceInterval;

    #[must_use]
    fn mul(self, rhs: ConfidenceInterval) -> Self::Output {
        rhs * self
    }
}

impl Add<f64> for ConfidenceInterval {
    type Output = Self;

    #[must_use]
    fn add(self, rhs: f64) -> Self::Output {
        Self::Output {
            mean: self.mean + rhs,
            margin: self.margin,
        }
    }
}

impl Add<ConfidenceInterval> for f64 {
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
pub const Z_90: f64 = 1.645;

#[allow(dead_code)]
pub const Z_85: f64 = 1.44;
