use mongodb::bson;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
pub struct TankSnapshot {
    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,
}
