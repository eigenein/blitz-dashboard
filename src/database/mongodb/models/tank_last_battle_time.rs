use std::borrow::Borrow;

use mongodb::bson;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::*;
use crate::wargaming;

#[derive(Copy, Clone)]
pub struct TankLastBattleTime {
    pub tank_id: wargaming::TankId,
    pub last_battle_time: DateTime,
}

impl<TS: Borrow<wargaming::TankStats>> From<TS> for TankLastBattleTime {
    fn from(tank_stats: TS) -> Self {
        let tank_stats = tank_stats.borrow();
        Self {
            tank_id: tank_stats.tank_id,
            last_battle_time: tank_stats.last_battle_time,
        }
    }
}

impl Serialize for TankLastBattleTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
        (self.tank_id, bson::DateTime::from(self.last_battle_time)).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TankLastBattleTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> StdResult<Self, D::Error> {
        let (tank_id, last_battle_time) =
            <(wargaming::TankId, bson::DateTime) as Deserialize>::deserialize(deserializer)?;
        Ok(Self {
            tank_id,
            last_battle_time: last_battle_time.into(),
        })
    }
}
