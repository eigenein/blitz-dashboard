pub type Vector = Vec<f64>;

#[must_use]
pub fn norm(x: &[f64]) -> f64 {
    x.iter().map(|xi| xi * xi).sum::<f64>().sqrt()
}

#[must_use]
#[inline]
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

#[cfg(test)]
#[cfg(nightly)]
mod benches {
    extern crate test;

    use test::{black_box, Bencher};

    use super::*;

    #[bench]
    fn bench_dot_8d(bencher: &mut Bencher) {
        let x = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];

        bencher.iter(|| black_box(dot(black_box(&x), black_box(&y))));
    }
}
