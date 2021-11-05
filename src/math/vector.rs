#[must_use]
pub fn norm(x: &[f64]) -> f64 {
    x.iter().map(|xi| xi * xi).sum::<f64>().sqrt()
}

#[must_use]
pub fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).fold(0.0, |dot, (xi, yi)| dot + xi * yi)
}

#[must_use]
pub fn cosine_similarity(x: &[f64], y: &[f64]) -> f64 {
    dot(x, y) / norm(x) / norm(y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_ok() {
        let vector_1 = [1.0, 2.0, 3.0];
        let vector_2 = [3.0, 5.0, 7.0];
        let similarity = cosine_similarity(&vector_1, &vector_2);
        assert!((similarity - 0.9974149030430578).abs() < f64::EPSILON);
    }
}
