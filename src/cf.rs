//! Collaborative filtering.

pub const N_FACTORS: usize = 9; // TODO: should be configurable.

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_factors(x: &mut Vec<f64>, length: usize) {
    x.truncate(length);
    while x.len() < length {
        x.push(fastrand::f64() - 0.5);
    }
}

pub fn predict_win_rate(vehicle_factors: &[f64], account_factors: &[f64]) -> f64 {
    const GLOBAL_BASELINE: f64 = 0.49;

    // TODO: clamp after each component.
    let prediction = GLOBAL_BASELINE
        + dot(
            account_factors,
            vehicle_factors,
            min_length(vehicle_factors, account_factors),
        );
    debug_assert!(!prediction.is_nan());
    prediction.clamp(0.0, 1.0)
}

/// Vector dot product.
#[must_use]
pub fn dot(x: &[f64], y: &[f64], length: usize) -> f64 {
    debug_assert!(x.len() <= length);
    debug_assert!(y.len() <= length);
    (0..length).map(|i| x[i] * y[i]).sum()
}

/// Subtracts the right vector from the left vector inplace.
/// The scaling is applied to the subtrahend.
pub fn subtract_vector(minuend: &mut [f64], subtrahend: &[f64], scaling: f64) {
    for i in 0..min_length(minuend, subtrahend) {
        minuend[i] -= scaling * subtrahend[i];
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
pub fn euclidean_distance(x: &[f64], y: &[f64]) -> f64 {
    x.iter()
        .zip(y)
        .map(|(xi, yi)| (xi - yi).powi(2))
        .sum::<f64>()
}

#[must_use]
pub fn euclidean_similarity(x: &[f64], y: &[f64]) -> f64 {
    -euclidean_distance(x, y)
}

#[must_use]
fn magnitude(x: &[f64], length: usize) -> f64 {
    debug_assert!(length <= x.len());
    x[..length]
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
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
