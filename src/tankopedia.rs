use std::collections::HashMap;

use static_init::dynamic;

use crate::models::{Nation, TankType, Vehicle};

#[dynamic]
static TANKOPEDIA: HashMap<i32, Vehicle> = {
    let tankopedia: HashMap<String, Vehicle> =
        serde_json::from_str(include_str!("tankopedia.json"))
            .expect("failed to parse the tankopedia");
    let mut tankopedia: HashMap<i32, Vehicle> = tankopedia
        .into_iter()
        .map(|(_, vehicle)| (vehicle.tank_id, vehicle))
        .collect();
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
    tankopedia
};

pub fn get_vehicle(tank_id: i32) -> Vehicle {
    TANKOPEDIA
        .get(&tank_id)
        .cloned() // FIXME: avoid `cloned()`.
        .unwrap_or_else(|| new_hardcoded_vehicle(tank_id))
}

fn new_hardcoded_vehicle(tank_id: i32) -> Vehicle {
    Vehicle {
        tank_id,
        name: format!("#{}", tank_id),
        tier: 0,
        is_premium: false,
        type_: TankType::Unknown,
        nation: Nation::Other,
    }
}
