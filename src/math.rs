pub mod traits;

#[allow(dead_code)]
#[inline]
pub fn logit(p: f64) -> f64 {
    (p / (1.0 - p)).ln()
}

#[allow(dead_code)]
#[inline]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
