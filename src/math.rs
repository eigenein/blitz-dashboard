#[allow(dead_code)]
#[must_use]
pub fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
#[allow(dead_code)]
pub fn ensure_vector_length(v: &mut Vec<f64>, length: usize) {
    v.truncate(length);
    while v.len() < length {
        v.push(fastrand::f64() - 0.5);
    }
}
