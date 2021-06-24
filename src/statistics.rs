/// `z*` for 90% confidence level.
pub const Z_90: f32 = 1.645;

pub struct ConfidenceInterval {
    pub mean: f32,
    pub margin: f32,
}

impl ConfidenceInterval {
    pub fn from_proportion(n_trials: i32, n_successes: i32, z: Option<f32>) -> Option<Self> {
        if n_trials == 0 {
            return None;
        }
        let n_trials = n_trials as f32;
        let n_successes = n_successes as f32;
        let mean = n_successes / n_trials;
        let margin = z.unwrap_or(Z_90) * (mean * (1.0 - mean) / n_trials).sqrt();
        Some(Self { mean, margin })
    }
}
