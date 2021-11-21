use serde::{Deserialize, Serialize};

/// Single sample point of a dataset.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SamplePoint {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_test: bool,
    pub n_battles: i32,
    pub n_wins: i32,
}
