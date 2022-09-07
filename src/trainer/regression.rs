use nalgebra::DVector;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Regression {
    pub bias: f64,
    pub k: f64,
    pub x: DVector<f64>,
    pub y: DVector<f64>,
    pub w: DVector<f64>,
}

impl Regression {
    pub fn predict(&self, x: f64) -> f64 {
        self.k * x + self.bias
    }
}
