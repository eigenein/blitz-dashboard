#[allow(dead_code)]
pub const Z_90: f64 = 1.645;

pub const Z_85: f64 = 1.44;

/// <https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval>
pub fn custom_wilson_score_interval(n_trials: i32, n_successes: i32, z: f64) -> (f64, f64) {
    let n_trials = n_trials as f64;
    let n_successes = n_successes as f64;

    let p_hat = n_successes / n_trials;

    let a = z * z / n_trials;
    let b = 1.0 / (1.0 + a);

    let p = b * (p_hat + a / 2.0);
    let margin = z * b * (p_hat * (1.0 - p_hat) / n_trials + a / n_trials / 4.0).sqrt();

    (p, margin)
}

pub fn wilson_score_interval(n_trials: i32, n_successes: i32) -> (f64, f64) {
    custom_wilson_score_interval(n_trials, n_successes, Z_85)
}
