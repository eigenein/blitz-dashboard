//! Collaborative filtering.

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_factors(x: &mut Vec<f64>, length: usize) {
    x.truncate(length);
    while x.len() < length {
        x.push(-1.0 + 2.0 * fastrand::f64());
    }
}

pub fn predict_win_rate(vehicle_factors: &[f64], account_factors: &[f64]) -> f64 {
    let length = min_length(vehicle_factors, account_factors);
    let prediction = dot(vehicle_factors, account_factors, length);
    assert!(!prediction.is_nan());
    logistic(prediction)
}

/// Vector dot product.
#[must_use]
pub fn dot(x: &[f64], y: &[f64], length: usize) -> f64 {
    debug_assert!(length <= x.len());
    debug_assert!(length <= y.len());
    (0..length).map(|i| x[i] * y[i]).sum()
}

/// Adjusts the latent factors.
/// See: https://sifter.org/~simon/journal/20061211.html.
///
/// ```java
/// userValue[user] += lrate * (err * movieValue[movie] - K * userValue[user]);
/// movieValue[movie] += lrate * (err * userValue[user] - K * movieValue[movie]);
/// ```
pub fn adjust_factors(
    left: &mut [f64],
    right: &[f64],
    error: f64,
    learning_rate: f64,
    regularization: f64,
) {
    debug_assert!(learning_rate >= 0.0);
    debug_assert!(regularization >= 0.0);
    debug_assert!(!error.is_nan());

    for i in 0..min_length(left, right) {
        left[i] += learning_rate * (error * right[i] - regularization * left[i]);
    }
}

/// https://en.wikipedia.org/wiki/Pearson_correlation_coefficient#For_a_sample
#[must_use]
pub fn pearson_coefficient(x: &[f64], y: &[f64]) -> f64 {
    let length = min_length(x, y);
    covariance(x, y, length) / std(x, length) / std(y, length)
}

#[must_use]
pub fn cosine_similarity(x: &[f64], y: &[f64]) -> f64 {
    let length = min_length(x, y);
    dot(x, y, length) / magnitude(x, length) / magnitude(y, length)
}

#[must_use]
pub fn magnitude(x: &[f64], length: usize) -> f64 {
    debug_assert!(length <= x.len());
    x[..length]
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

#[must_use]
pub fn make_targets(n_battles: i32, n_wins: i32) -> Vec<f64> {
    let mut targets: Vec<f64> = (0..n_battles)
        .map(|i| if i < n_wins { 1.0 } else { 0.0 })
        .collect();
    fastrand::shuffle(&mut targets);
    targets
}

#[must_use]
fn mean(x: &[f64], length: usize) -> f64 {
    debug_assert!(length <= x.len());
    debug_assert_ne!(length, 0);
    x[..length].iter().sum::<f64>() / x.len() as f64
}

#[must_use]
fn covariance(x: &[f64], y: &[f64], length: usize) -> f64 {
    let x_mean = mean(x, length);
    let y_mean = mean(y, length);
    (0..length)
        .map(|i| (x[i] - x_mean) * (y[i] - y_mean))
        .sum::<f64>()
}

#[must_use]
fn std(x: &[f64], length: usize) -> f64 {
    let mean = mean(x, length);
    x[..length]
        .iter()
        .map(|xi| (xi - mean).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[must_use]
fn min_length(x: &[f64], y: &[f64]) -> usize {
    x.len().min(y.len())
}

#[must_use]
fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_targets_ok() {
        let sum: f64 = make_targets(100, 30).iter().sum();
        assert!(sum > 29.9 && sum < 30.1);
    }
}
