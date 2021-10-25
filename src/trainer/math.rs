//! Collaborative filtering.

use std::cmp::Ordering;

use rand::distributions::Distribution;
use rand::thread_rng;
use rand_distr::Normal;

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
        let old_xi = *xi;
        *xi += residual_multiplier * *yi - regularization_multiplier * *xi;
        *yi += residual_multiplier * old_xi - regularization_multiplier * *yi;
    }
}

#[must_use]
pub fn norm(x: &[f64]) -> f64 {
    x.iter().map(|xi| xi * xi).sum::<f64>().sqrt()
}

#[must_use]
pub fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(xi, yi)| xi * yi).sum()
}

#[must_use]
pub fn cosine_similarity(x: &[f64], y: &[f64]) -> f64 {
    dot(x, y) / norm(x) / norm(y)
}

#[must_use]
fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
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
