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
pub fn ensure_vector_length(v: &mut Vec<f64>, length: usize) {
    v.truncate(length);
    while v.len() < length {
        v.push(fastrand::f64() - 0.5);
    }
}

pub fn add_vector(to: &mut [f64], vector: &[f64], scaling: f64) {
    assert_eq!(to.len(), vector.len());
    for i in 0..vector.len() {
        to[i] += scaling * vector[i];
    }
}
