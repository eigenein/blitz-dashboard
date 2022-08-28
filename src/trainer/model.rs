use crate::prelude::*;

#[derive(Default)]
pub struct Model {
    pub vehicles: AHashMap<wargaming::TankId, VehicleModel>,
}

#[derive(Default)]
pub struct VehicleModel {
    pub mean_rating: f64,
    pub similarities: AHashMap<wargaming::TankId, f64>,
}
