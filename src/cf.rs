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

#[must_use]
fn magnitude(x: &[f64]) -> f64 {
    x.iter().map(|value| value * value).sum::<f64>().sqrt()
}

#[must_use]
fn cosine_similarity(x: &[f64], y: &[f64]) -> f64 {
    debug_assert_eq!(
        x.len(),
        y.len(),
        "trying to calculate a cosine similarity for vectors of sizes {} and {}",
        x.len(),
        y.len(),
    );
    dot(x, y) / magnitude(x) / magnitude(y)
}

#[must_use]
fn mean(vector: &[f64]) -> f64 {
    debug_assert!(
        !vector.is_empty(),
        "trying to calculate a mean for an empty vector",
    );
    vector.iter().sum::<f64>() / vector.len() as f64
}

/// https://en.wikipedia.org/wiki/Pearson_correlation_coefficient#For_a_sample
#[must_use]
fn pearson_coefficient(x: &[f64], y: &[f64]) -> f64 {
    covariance(x, y) / std(x) / std(y)
}

#[must_use]
fn covariance(x: &[f64], y: &[f64]) -> f64 {
    debug_assert_eq!(
        x.len(),
        y.len(),
        "trying to calculate a covariance for vectors of sizes {} and {}",
        x.len(),
        y.len(),
    );

    let x_mean = mean(x);
    let y_mean = mean(y);

    x.iter()
        .zip(y)
        .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
        .sum()
}

#[must_use]
fn std(x: &[f64]) -> f64 {
    debug_assert_ne!(
        x.len(),
        1,
        "standard deviation is not defined for a vector of length 1",
    );

    let mean = mean(x);
    x.iter().map(|xi| (xi - mean).powi(2)).sum::<f64>().sqrt()
}
