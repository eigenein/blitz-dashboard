//! Collaborative filtering.

use crate::trainer::vector::Vector;

/// Truncates the vector, if needed.
/// Pushes random values to it until the target length is reached.
pub fn initialize_factors(x: &mut Vector, length: usize, hard: bool) {
    x.0.truncate(length);
    while x.0.len() < length {
        let factor = if hard {
            // Used to generate the factors from scratch.
            // Generate a factor as a random value from 0.05…0.10.
            0.05 + 0.05 * fastrand::f64()
        } else {
            // Used to generate additional factors for a pre-trained model.
            0.001
        };
        x.0.push(if fastrand::bool() { factor } else { -factor });
    }
}

pub fn predict_win_rate(vehicle_factors: &Vector, account_factors: &Vector) -> f64 {
    let prediction = vehicle_factors.dot(account_factors);
    assert!(!prediction.is_nan());
    logistic(prediction)
}

/// Adjusts the latent factors.
/// See: https://sifter.org/~simon/journal/20061211.html.
pub fn adjust_factors(
    left: &mut Vector,
    right: &Vector,
    residual_error: f64,
    learning_rate: f64,
    regularization: f64,
) {
    debug_assert!(learning_rate >= 0.0);
    debug_assert!(regularization >= 0.0);
    assert!(!residual_error.is_nan());

    // userValue[user] += lrate * (err * movieValue[movie] - K * userValue[user]);
    // movieValue[movie] += lrate * (err * userValue[user] - K * movieValue[movie]);
    left.add_assign(
        right
            .mul(residual_error)
            .sub(&left.mul(regularization))
            .mul(learning_rate),
    );
}

#[must_use]
fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
