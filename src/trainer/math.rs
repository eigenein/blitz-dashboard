//! Collaborative filtering.

use anyhow::anyhow;
use std::cmp::Ordering;

use rand::distributions::Distribution;
use rand::thread_rng;
use rand_distr::Normal;

use crate::math::logistic;
use crate::math::vector::dot;
use crate::Vector;

pub fn initialize_factors(x: &mut Vector, length: usize, magnitude: f64) -> bool {
    match x.len().cmp(&length) {
        Ordering::Equal => false,
        _ => {
            let mut rng = thread_rng();
            let distribution = Normal::new(0.0, magnitude).unwrap();
            x.clear();
            while x.len() < length {
                x.push(distribution.sample(&mut rng));
            }
            true
        }
    }
}

#[must_use]
#[inline]
pub fn predict_win_rate(vehicle_factors: &[f64], account_factors: &[f64]) -> f64 {
    logistic(dot(vehicle_factors, account_factors))
}

/// Adjusts the latent factors.
/// See: https://sifter.org/~simon/journal/20061211.html.
#[inline]
pub fn sgd(
    x: &mut [f64],
    y: &mut [f64],
    residual_multiplier: f64,
    regularization_multiplier: f64,
) -> crate::Result {
    for (xi, yi) in x.iter_mut().zip(y.iter_mut()) {
        *xi += residual_multiplier * *yi - regularization_multiplier * *xi;
        *yi += residual_multiplier * *xi - regularization_multiplier * *yi;

        if !xi.is_finite() || !yi.is_finite() {
            return Err(anyhow!("the learning rate is too big"));
        }
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
            black_box(sgd(
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

        bencher.iter(|| black_box(predict_win_rate(black_box(&x), black_box(&y))));
    }
}
