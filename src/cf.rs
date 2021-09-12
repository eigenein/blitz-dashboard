//! Collaborative filtering.

pub const N_FACTORS: usize = 9;

/// Vector dot product.
#[must_use]
pub fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(left, right)| left * right).sum()
}

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_factors(x: &mut Vec<f64>, length: usize) {
    x.truncate(length);
    while x.len() < length {
        x.push(fastrand::f64() - 0.5);
    }
}

/// Subtracts the right vector from the left vector inplace.
/// The scaling is applied to the subtrahend.
pub fn subtract_vector(minuend: &mut [f64], subtrahend: &[f64], scaling: f64) {
    debug_assert_eq!(
        minuend.len(),
        subtrahend.len(),
        "trying to subtract a vector of length {} from a vector of length {}",
        subtrahend.len(),
        minuend.len(),
    );
    for i in 0..subtrahend.len() {
        minuend[i] -= scaling * subtrahend[i];
    }
}

/// Note: vehicle bias is the 0-th element in the factor array.
pub fn predict_win_rate(
    global_bias: f64,
    vehicle_factors: &[f64],
    account_bias: f64,
    account_factors: &[f64],
) -> f64 {
    if account_factors.is_empty() || vehicle_factors.is_empty() {
        return 0.5; // FIXME.
    }

    let prediction = global_bias
        + account_bias
        + vehicle_factors[0] // vehicle bias
        + dot(account_factors, &vehicle_factors[1..]);
    debug_assert!(!prediction.is_nan());
    prediction.clamp(0.0, 1.0)
}

/// https://en.wikipedia.org/wiki/Pearson_correlation_coefficient#For_a_sample
#[must_use]
pub fn pearson_coefficient(x: &[f64], y: &[f64]) -> f64 {
    let length = min_length(x, y);
    covariance(x, y) / std(x, length) / std(y, length)
}

#[must_use]
pub fn cosine_similarity(x: &[f64], y: &[f64]) -> f64 {
    let length = min_length(x, y);
    dot(x, y) / magnitude(x, length) / magnitude(y, length)
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
    debug_assert_ne!(length, 0, "the specified length is zero");
    x[..length].iter().sum::<f64>() / x.len() as f64
}

#[must_use]
fn covariance(x: &[f64], y: &[f64]) -> f64 {
    let length = min_length(x, y);
    let x_mean = mean(x, length);
    let y_mean = mean(y, length);

    x.iter()
        .zip(y)
        .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
        .sum()
}

#[must_use]
fn std(x: &[f64], length: usize) -> f64 {
    debug_assert!(length <= x.len());
    debug_assert_ne!(
        length, 1,
        "standard deviation is not defined for a vector of length 1",
    );

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
