use mongodb::bson;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::*;
use crate::wargaming;

#[derive(Copy, Clone)]
pub struct TankLastBattleTime {
    pub tank_id: wargaming::TankId,
    pub last_battle_time: DateTime,
}

impl From<&wargaming::TankStats> for TankLastBattleTime {
    fn from(tank_stats: &wargaming::TankStats) -> Self {
        Self {
            tank_id: tank_stats.tank_id,
            last_battle_time: tank_stats.last_battle_time,
        }
    }
}

// TODO: use `serde_with::TryFromInto` instead.
impl Serialize for TankLastBattleTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ((self.tank_id as i32), bson::DateTime::from(self.last_battle_time)).serialize(serializer)
    }
}

// TODO: use `serde_with::TryFromInto` instead.
impl<'de> Deserialize<'de> for TankLastBattleTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (tank_id, last_battle_time) =
            <(wargaming::TankId, bson::DateTime) as Deserialize>::deserialize(deserializer)?;
        Ok(Self {
            tank_id,
            last_battle_time: last_battle_time.into(),
        })
    }
}
