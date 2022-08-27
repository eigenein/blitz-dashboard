use serde::{Deserialize, Serialize};

use crate::wargaming;

#[derive(Serialize, Deserialize)]
pub struct RecommendRequest {
    pub given: Vec<(wargaming::TankId, f64)>,
    pub predict: Vec<wargaming::TankId>,
}
