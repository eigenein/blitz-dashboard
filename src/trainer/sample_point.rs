use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::DateTime;

/// Single sample point of a dataset.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SamplePoint {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_test: bool,
    pub n_battles: i32,
    pub n_wins: i32,

    #[serde(default = "Utc::now")]
    pub timestamp: DateTime,
}
