use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::wargaming::models::TankId;

#[derive(Serialize, Deserialize, Clone, Debug, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum Nation {
    #[serde(rename = "ussr")]
    Ussr,

    #[serde(rename = "germany")]
    Germany,

    #[serde(rename = "usa")]
    Usa,

    #[serde(rename = "china")]
    China,

    #[serde(rename = "france")]
    France,

    #[serde(rename = "uk")]
    Uk,

    #[serde(rename = "japan")]
    Japan,

    #[serde(rename = "european")]
    Europe,

    #[serde(other, rename = "other")]
    Other,
}

impl Nation {
    /// Construct `Nation` from the API tank ID.
    pub fn from_tank_id(tank_id: TankId) -> Result<Nation> {
        let tank_id = tank_id;
        const NATIONS: &[Nation] = &[
            Nation::Ussr,
            Nation::Germany,
            Nation::Usa,
            Nation::China,
            Nation::France,
            Nation::Uk,
            Nation::Japan,
            Nation::Other,
            Nation::Europe,
        ];

        const COMPONENT_VEHICLE: TankId = 1;
        debug_assert_eq!(tank_id & COMPONENT_VEHICLE, COMPONENT_VEHICLE);

        let nation = ((tank_id >> 4) & 0xF) as usize;
        NATIONS
            .get(nation)
            .copied()
            .ok_or_else(|| anyhow!("unexpected nation {} for tank {}", nation, tank_id))
    }

    /// Get the client nation ID.
    pub const fn get_id(self) -> u32 {
        match self {
            Nation::Ussr => 20000,
            Nation::Germany => 30000,
            Nation::Usa => 10000,
            Nation::China => 60000,
            Nation::France => 70000,
            Nation::Uk => 40000,
            Nation::Japan => 50000,
            Nation::Other => 100000,
            Nation::Europe => 80000,
        }
    }
}
