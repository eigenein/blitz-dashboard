//! Collaborative filtering.

use std::cmp::Ordering;

use rand::distributions::Distribution;
use rand::thread_rng;
use rand_distr::Normal;

use crate::trainer::vector::Vector;

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_factors(x: &mut Vector, length: usize, magnitude: f64) -> bool {
    match x.0.len().cmp(&length) {
        Ordering::Equal => false,
        Ordering::Less => {
            let mut rng = thread_rng();
            let distribution = Normal::new(0.0, magnitude).unwrap();
            while x.0.len() < length {
                x.0.push(distribution.sample(&mut rng));
            }
            true
        }
        Ordering::Greater => {
            x.0.truncate(length);
            true
        }
    }
}

pub fn predict_win_rate(vehicle_factors: &Vector, account_factors: &Vector) -> f64 {
    let prediction = vehicle_factors.dot(account_factors);
    assert!(!prediction.is_nan());
    prediction.clamp(0.0, 1.0)
}
