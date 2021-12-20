use serde::{Deserialize, Serialize};

use crate::DateTime;

/// Single sample point of a dataset.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SamplePoint {
    pub account_id: i32,
    pub tank_id: i32,
    pub is_test: bool,
    pub is_win: bool,
    pub timestamp: DateTime,

    /// Being phased out.
    pub n_battles: i32,

    /// Being phased out.
    pub n_wins: i32,
}
