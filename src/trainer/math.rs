//! Collaborative filtering.

use crate::math::logistic;
use crate::math::vector::dot;

/// Predict a probability based on the user and item latent vectors.
#[must_use]
#[inline]
pub fn predict_probability(x: &[f64], y: &[f64]) -> f64 {
    logistic(dot(x, y))
}

/// Adjusts the latent factors and returns the updated dot product.
///
/// See also: https://sifter.org/~simon/journal/20061211.html.
#[inline]
pub fn make_gradient_descent_step(
    x: &mut [f64],
    y: &mut [f64],
    residual_error: f64,
    regularization: f64,
    x_learning_rate: f64,
    y_learning_rate: f64,
) -> f64 {
    let mut dot = 0.0;
    for (xi, yi) in x.iter_mut().zip(y.iter_mut()) {
        *xi += x_learning_rate * (residual_error * *yi - regularization * *xi);
        *yi += y_learning_rate * (residual_error * *xi - regularization * *yi);
        dot += *xi * *yi;
    }
    dot
}

#[cfg(test)]
#[cfg(nightly)]
mod benches {
    extern crate test;

    use test::{black_box, Bencher};

    use super::*;

    #[bench]
    fn bench_predict_win_rate_8d(bencher: &mut Bencher) {
        let x = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];

        bencher.iter(|| black_box(predict_probability(black_box(&x), black_box(&y))));
    }
}
