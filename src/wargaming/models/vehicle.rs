use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::wargaming::models::{Nation, TankId};

/// Represents a generic vehicle from the tankopedia.
#[derive(Serialize, Deserialize, Clone)]
pub struct Vehicle {
    pub tank_id: TankId,
    pub name: Cow<'static, str>,
    pub tier: i32,
    pub is_premium: bool,
    pub nation: Nation,

    #[serde(rename = "type")]
    pub type_: TankType,
}

impl Vehicle {
    /// Creates a fake vehicle instance with the specified ID.
    pub fn new_hardcoded(tank_id: TankId) -> Self {
        Self {
            tank_id,
            name: Cow::Owned(format!("#{}", tank_id)),
            tier: 0,
            is_premium: false,
            type_: TankType::Unknown,
            nation: Nation::from_tank_id(tank_id).unwrap_or(Nation::Other),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum TankType {
    #[serde(rename = "lightTank")]
    Light,

    #[serde(rename = "mediumTank")]
    Medium,

    #[serde(rename = "heavyTank")]
    Heavy,

    #[serde(rename = "AT-SPG")]
    AT,

    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::prelude::*;

    #[test]
    fn tankopedia_ok() -> Result {
        serde_json::from_str::<HashMap<String, Vehicle>>(
            // language=json
            r#"{"1649":{"suspensions":[1138],"description":"Неостановимый Дракула возродился, и тьма нависла над миром. Долг зовёт охотника на вампиров Хелсинга вновь встать на защиту Света и дать бой древнему злу. Воплощение Хелсинга — это произведение искусства, инкрустированная защитными орнаментами боевая машина, снаряжённая специально для борьбы с порождениями тьмы. Сдвоенное орудие Хелсинга стреляет два раза автоматически — только так можно остановить полёт Дракулы и одержать победу.\r\nПремиум танк «Хелсинг H0» можно было получить во время игрового события «Ночная охота» в октябре 2016 года.","engines":[17013],"prices_xp":null,"next_tanks":null,"modules_tree":{"1138":{"name":"Helsing type1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1138,"type":"vehicleChassis"},"1139":{"name":"Helsing type1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1139,"type":"vehicleTurret"},"1140":{"name":"85mm Twin X-Barrel mod1","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":1140,"type":"vehicleGun"},"17013":{"name":"Aether W-20","next_modules":null,"next_tanks":null,"is_default":true,"price_xp":0,"price_credit":0,"module_id":17013,"type":"vehicleEngine"}},"nation":"other","is_premium":true,"images":{"preview":"http:\/\/glossary-ru-static.gcdn.co\/icons\/wotb\/current\/uploaded\/vehicles\/hd_thumbnail\/Van_Helsing.png","normal":"http:\/\/glossary-ru-static.gcdn.co\/icons\/wotb\/current\/uploaded\/vehicles\/hd\/Van_Helsing.png"},"cost":null,"default_profile":{"weight":24880,"profile_id":"1138-1139-1140-17013","firepower":62,"shot_efficiency":67,"gun_id":1140,"signal_range":null,"shells":[{"type":"ARMOR_PIERCING","penetration":170,"damage":200},{"type":"ARMOR_PIERCING_CR","penetration":220,"damage":170},{"type":"HIGH_EXPLOSIVE","penetration":45,"damage":300}],"armor":{"turret":{"front":80,"sides":50,"rear":40},"hull":{"front":60,"sides":40,"rear":40}},"speed_forward":60,"battle_level_range_min":7,"speed_backward":15,"engine":{"tier":8,"fire_chance":0.2,"power":500,"name":"Aether W-20","weight":530},"max_ammo":100,"battle_level_range_max":8,"engine_id":17013,"hp":1000,"is_default":true,"protection":30,"suspension":{"tier":7,"load_limit":27800,"traverse_speed":30,"name":"Helsing type1","weight":6000},"suspension_id":1138,"max_weight":27800,"gun":{"move_down_arc":6,"caliber":85,"name":"85mm Twin X-Barrel mod1","weight":3800,"move_up_arc":15,"fire_rate":11.71,"clip_reload_time":0.25,"dispersion":0.34,"clip_capacity":2,"traverse_speed":43.75,"reload_time":10.0,"tier":8,"aim_time":4.2},"turret_id":1139,"turret":{"name":"Helsing type1","weight":3350,"view_range":240,"traverse_left_arc":180,"hp":200,"traverse_speed":17,"tier":7,"traverse_right_arc":180},"maneuverability":53,"hull_weight":10950,"hull_hp":800},"tier":7,"tank_id":1649,"type":"AT-SPG","guns":[1140],"turrets":[1139],"name":"Helsing"}}"#,
        )?;
        Ok(())
    }
}
