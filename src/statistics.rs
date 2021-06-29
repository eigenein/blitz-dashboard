/// `z*` for 90% confidence level.
pub const Z_90: f64 = 1.645;

pub struct ConfidenceInterval {
    pub mean: f64,
    pub margin: f64,
}

impl ConfidenceInterval {
    pub fn from_proportion(n_trials: i32, n_successes: i32, z: f64) -> Option<Self> {
        if n_trials == 0 {
            return None;
        }
        let n_trials = n_trials as f64;
        let n_successes = n_successes as f64;
        let mean = n_successes / n_trials;
        let margin = z * (mean * (1.0 - mean) / n_trials).sqrt();
        Some(Self { mean, margin })
    }

    pub fn from_proportion_90(n_trials: i32, n_successes: i32) -> Option<Self> {
        Self::from_proportion(n_trials, n_successes, Z_90)
    }

    pub fn get_percentages(&self) -> (f64, f64, f64) {
        let mean = self.mean * 100.0;
        let margin = self.margin * 100.0;
        let lower = (mean - margin).max(0.0);
        let upper = (mean + margin).min(100.0);
        (lower, mean, upper)
    }
}
