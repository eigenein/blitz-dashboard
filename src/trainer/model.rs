use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Default)]
pub struct Model {
    pub regressions: AHashMap<
        wargaming::Realm,
        AHashMap<wargaming::TankId, AHashMap<wargaming::TankId, Regression>>,
    >,
}

#[derive(Serialize, Deserialize)]
pub struct Regression {
    pub bias: f64,
    pub k: f64,
    pub x: DVector<f64>,
    pub y: DVector<f64>,
}

impl Regression {
    pub fn predict(&self, x: f64) -> f64 {
        self.k * x + self.bias
    }
}
