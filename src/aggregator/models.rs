use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::math::statistics::ConfidenceInterval;
use crate::wargaming::tank_id::TankId;

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Analytics {
    pub time_spans: Vec<DurationWrapper>,
    pub win_rates: Vec<(TankId, Vec<Option<ConfidenceInterval>>)>,
}

#[derive(Serialize, Deserialize)]
pub struct DurationWrapper {
    #[serde(
        serialize_with = "crate::helpers::serde::serialize_duration_seconds",
        deserialize_with = "crate::helpers::serde::deserialize_duration_seconds"
    )]
    pub duration: Duration,
}

#[derive(Default, Serialize, Deserialize)]
pub struct BattleCount {
    pub n_wins: i32,
    pub n_battles: i32,
}
