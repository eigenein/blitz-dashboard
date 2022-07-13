use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::wargaming::TankId;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TankAchievements {
    pub tank_id: TankId,
    pub achievements: HashMap<String, i32>,
    pub max_series: HashMap<String, i32>,
}

pub struct TanksAchievements(pub Vec<TankAchievements>);

impl Serialize for TanksAchievements {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TanksAchievements {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Deserialize::deserialize(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn tank_achievements_ok() -> Result {
        serde_json::from_str::<HashMap<String, TanksAchievements>>(
            // language=json
            r#"{"5589968":[{"achievements":{"medalCarius":4,"medalLehvaslaiho":1,"medalAbrams":4,"armorPiercer":1,"medalPoppel":4,"markOfMasteryII":6,"supporter":1,"medalKay":4,"warrior":2,"mainGun":2,"titleSniper":1,"markOfMasteryIII":4,"medalKnispel":4},"max_series":{"armorPiercer":20,"punisher":0,"titleSniper":21,"invincible":1,"tankExpert":0,"medalKay":5,"diehard":2,"beasthunter":1,"handOfDeath":2,"jointVictory":0,"sinai":0,"pattonValley":0},"account_id":5589968,"tank_id":1}]}"#,
        )?;
        Ok(())
    }
}
