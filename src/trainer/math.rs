//! Collaborative filtering.

use crate::math::logistic;
use crate::math::vector::dot;

/// Predict a probability based on the user and item latent vectors.
/// The 0-th elements are biases.
#[must_use]
#[inline]
pub fn predict_probability(x: &[f64], y: &[f64]) -> f64 {
    logistic(x[0] + y[0] + dot(&x[1..], &y[1..]))
}

/// Adjusts the latent factors.
///
/// Note that the 0-th elements of the vectors are biases.
///
/// See also: https://sifter.org/~simon/journal/20061211.html.
#[inline]
pub fn make_gradient_descent_step(
    x: &mut [f64],
    y: &mut [f64],
    residual_multiplier: f64,
    regularization_multiplier: f64,
) -> crate::Result {
    // Adjust the biases separately.
    x[0] += residual_multiplier - regularization_multiplier * x[0];
    y[0] += residual_multiplier - regularization_multiplier * y[0];

    for (xi, yi) in x[1..].iter_mut().zip(y[1..].iter_mut()) {
        *xi += residual_multiplier * *yi - regularization_multiplier * *xi;
        *yi += residual_multiplier * *xi - regularization_multiplier * *yi;
    }

    Ok(())
}

#[cfg(test)]
#[cfg(nightly)]
mod benches {
    extern crate test;

    use super::*;
    use test::{black_box, Bencher};

    #[bench]
    fn bench_sgd_3d(bencher: &mut Bencher) {
        let mut x = vec![0.1, 0.2, 0.3];
        let mut y = vec![-0.1, -0.2, -0.3];

        bencher.iter(|| {
            black_box(make_gradient_descent_step(
                black_box(&mut x),
                black_box(&mut y),
                black_box(0.00001 * 0.5),
                black_box(0.00001 * 0.1),
            ))
        });
    }

    #[bench]
    fn bench_predict_win_rate_8d(bencher: &mut Bencher) {
        let x = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0];

        bencher.iter(|| black_box(predict_probability(black_box(&x), black_box(&y))));
    }
}
