//! Collaborative filtering.

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
pub fn predict_win_rate(vehicle_factors: &[f64], account_factors: &[f64]) -> f64 {
    logistic(dot(vehicle_factors, account_factors))
}

/// Adjusts the latent factors.
/// See: https://sifter.org/~simon/journal/20061211.html.
pub fn sgd(
    x: &mut [f64],
    y: &mut [f64],
    prediction: f64,
    target: f64,
    learning_rate: f64,
    regularization: f64,
) {
    let residual_multiplier = learning_rate * (target - prediction);
    let regularization_multiplier = learning_rate * regularization;

    for (xi, yi) in x.iter_mut().zip(y.iter_mut()) {
        *xi += residual_multiplier * *yi - regularization_multiplier * *xi;
        *yi += residual_multiplier * *xi - regularization_multiplier * *yi;
    }
}
