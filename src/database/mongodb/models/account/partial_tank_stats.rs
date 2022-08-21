use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct PartialTankStats {
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "tid")]
    pub tank_id: wargaming::TankId,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nb")]
    pub n_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nw")]
    pub n_wins: u32,
}

impl From<&wargaming::TankStats> for PartialTankStats {
    fn from(tank_stats: &wargaming::TankStats) -> Self {
        Self {
            tank_id: tank_stats.tank_id,
            n_battles: tank_stats.all.n_battles,
            n_wins: tank_stats.all.n_wins,
        }
    }
}
