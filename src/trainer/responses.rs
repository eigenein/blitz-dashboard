use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::wargaming;

#[derive(Serialize, Deserialize, Default)]
pub struct RecommendResponse {
    pub predictions: Vec<Prediction>,
}

#[derive(Serialize, Deserialize)]
pub struct Prediction {
    pub tank_id: wargaming::TankId,
    pub p: f64,
    pub n_sources: usize,
}

impl Prediction {
    pub const fn new(tank_id: wargaming::TankId) -> Self {
        Self {
            tank_id,
            p: 0.0,
            n_sources: 0,
        }
    }
}

impl Eq for Prediction {}

impl PartialEq<Self> for Prediction {
    #[inline]
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl PartialOrd<Self> for Prediction {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Prediction {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.p.total_cmp(&other.p)
    }
}
