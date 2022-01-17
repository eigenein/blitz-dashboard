use crate::DateTime;
use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::math::statistics::ConfidenceInterval;
use crate::wargaming::tank_id::TankId;

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Analytics {
    pub time_spans: Vec<DurationWrapper>,
    pub win_rates: Vec<(TankId, Vec<Option<ConfidenceInterval>>)>, // FIXME: introduce a type.
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
pub struct BattleCounts {
    pub n_wins: i32,
    pub n_battles: i32,
}

pub struct VehicleEntry {
    pub timestamp: DateTime,
    pub battle_counts: BattleCounts,
}
