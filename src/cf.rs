//! Collaborative filtering.

pub const N_FACTORS: usize = 8;
const GLOBAL_BIAS: f64 = 0.5;

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

pub fn predict_win_rate(
    vehicle_bias: f64,
    vehicle_factors: &[f64],
    account_bias: f64,
    account_factors: &[f64],
) -> f64 {
    if vehicle_factors.len() != account_factors.len() {
        // FIXME.
        return 0.0;
    }
    let prediction =
        GLOBAL_BIAS + account_bias + vehicle_bias + dot(account_factors, vehicle_factors);
    assert!(!prediction.is_nan());
    if prediction < 0.0 {
        0.0
    } else if prediction > 1.0 {
        1.0
    } else {
        prediction
    }
}
