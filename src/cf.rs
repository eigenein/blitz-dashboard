//! Collaborative filtering.

pub const N_FACTORS: usize = 8;

/// Vector dot product.
#[must_use]
pub fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_factors(v: &mut Vec<f64>, length: usize) {
    v.truncate(length);
    while v.len() < length {
        v.push(fastrand::f64() - 0.5);
    }
}

/// Subtracts the right vector from the left vector inplace.
/// The scaling is applied to the subtrahend.
pub fn subtract_vector(minuend: &mut [f64], subtrahend: &[f64], scaling: f64) {
    assert_eq!(minuend.len(), subtrahend.len());
    for i in 0..subtrahend.len() {
        minuend[i] -= scaling * subtrahend[i];
    }
}

/// Note: vehicle bias is the 0-th element in the factor array.
pub fn predict_win_rate(
    vehicle_factors: &[f64],
    account_bias: f64,
    account_factors: &[f64],
) -> f64 {
    const GLOBAL_BIAS: f64 = 0.5;

    if account_factors.is_empty() || vehicle_factors.is_empty() {
        return 0.5; // FIXME.
    }

    let prediction = GLOBAL_BIAS
        + account_bias
        + vehicle_factors[0] // vehicle bias
        + dot(account_factors, &vehicle_factors[1..]);
    assert!(!prediction.is_nan());
    prediction.clamp(0.0, 1.0)
}
