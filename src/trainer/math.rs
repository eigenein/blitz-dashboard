//! Collaborative filtering.

use std::cmp::Ordering;

use rand::distributions::Distribution;
use rand::thread_rng;
use rand_distr::Normal;

use crate::trainer::vector::Vector;

pub fn initialize_factors(x: &mut Vector, length: usize, magnitude: f64) -> bool {
    match x.0.len().cmp(&length) {
        Ordering::Equal => false,
        _ => {
            let mut rng = thread_rng();
            let distribution = Normal::new(0.0, magnitude).unwrap();
            x.0.clear();
            while x.0.len() < length {
                x.0.push(distribution.sample(&mut rng));
            }
            true
        }
    }
}

pub fn predict_win_rate(vehicle_factors: &Vector, account_factors: &Vector) -> f64 {
    let prediction = vehicle_factors.dot(account_factors);
    assert!(!prediction.is_nan());
    prediction
}

/// Adjusts the latent factors.
/// See: https://sifter.org/~simon/journal/20061211.html.
pub fn sgd(
    x: &mut Vector,
    y: &mut Vector,
    residual_error: f64,
    learning_rate: f64,
    regularization: f64,
) {
    debug_assert!(learning_rate >= 0.0);
    debug_assert!(regularization >= 0.0);

    let residual_multiplier = learning_rate * residual_error;
    let regularization_multiplier = learning_rate * regularization;
    for (xi, yi) in x.0.iter_mut().zip(y.0.iter_mut()) {
        let old_xi = *xi;
        *xi += residual_multiplier * *yi - regularization_multiplier * *xi;
        *yi += residual_multiplier * old_xi - regularization_multiplier * *yi;
    }
}
