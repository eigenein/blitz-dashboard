use serde::Serialize;

use crate::prelude::*;

#[derive(Default)]
pub struct Model {
    pub regressions: AHashMap<
        wargaming::Realm,
        AHashMap<wargaming::TankId, AHashMap<wargaming::TankId, Regression>>,
    >,
}

#[derive(Serialize)]
pub struct Regression {
    pub bias: f64,
    pub k: f64,
    pub n_rows: usize,
}

impl Regression {
    pub fn predict(&self, x: f64) -> f64 {
        self.k * x + self.bias
    }
}
