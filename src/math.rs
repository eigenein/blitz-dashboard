pub mod statistics;
pub mod vector;

#[must_use]
pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
