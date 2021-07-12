use std::collections::HashMap;

use crate::models::{Nation, TankType, Vehicle};
use crate::wargaming::WargamingApi;

pub struct Tankopedia(HashMap<i32, Vehicle>);

impl Tankopedia {
    pub async fn new(api: &WargamingApi) -> crate::Result<Self> {
        let mut tankopedia = api.get_tankopedia().await?;
        tankopedia.insert(
            23057,
            Vehicle {
                tank_id: 23057,
                name: "Kunze Panzer".to_string(),
                tier: 7,
                is_premium: true,
                nation: Nation::Germany,
                type_: TankType::Light,
            },
        );
        tankopedia.insert(
            20817,
            Vehicle {
                tank_id: 20817,
                name: "Эксплорер".to_string(),
                tier: 6,
                is_premium: true,
                nation: Nation::Uk,
                type_: TankType::Medium,
            },
        );
        Ok(Self(tankopedia))
    }

    pub fn get(&self, tank_id: i32) -> Vehicle {
        self.0
            .get(&tank_id)
            .cloned() // FIXME: avoid `cloned()`.
            .unwrap_or_else(|| Self::new_hardcoded_vehicle(tank_id))
    }

    fn new_hardcoded_vehicle(tank_id: i32) -> Vehicle {
        Vehicle {
            tank_id,
            name: format!("#{}", tank_id),
            tier: 0,
            is_premium: false,
            type_: TankType::Light, // FIXME
            nation: Nation::Other,
        }
    }
}
