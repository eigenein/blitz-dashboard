#[must_use]
pub fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_random_vector(v: &mut Vec<f64>, length: usize) {
    v.truncate(length);
    while v.len() < length {
        v.push(fastrand::f64() - 0.5);
    }
}

pub fn subtract_vector(minuend: &mut [f64], subtrahend: &[f64], scaling: f64) {
    assert_eq!(minuend.len(), subtrahend.len());
    for i in 0..subtrahend.len() {
        minuend[i] -= scaling * subtrahend[i];
    }
}
