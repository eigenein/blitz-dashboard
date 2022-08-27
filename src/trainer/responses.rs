use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct VehicleResponse {
    pub victory_ratio: f64,
    pub similar_vehicles: Vec<(wargaming::TankId, f64)>,
}
