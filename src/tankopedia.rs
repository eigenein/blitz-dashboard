use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::models::{Nation, TankType, Vehicle};
use crate::opts::ImportTankopediaOpts;
use crate::wargaming::{Tankopedia, WargamingApi};

mod generated;

/// Retrieves a vehicle from the Tankopedia.
pub fn get_vehicle(tank_id: i32) -> Cow<'static, Vehicle> {
    generated::GENERATED
        .get(&tank_id)
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(Vehicle::new_hardcoded(tank_id)))
}

/// Some vehicles are just copies of another vehicles.
/// Maps a tank ID to its original vehicle.
#[must_use]
#[inline]
pub fn remap_tank_id(tank_id: i32) -> i32 {
    match tank_id {
        64273 => 55313, // 8,8 cm Pak 43 Jagdtiger
        64769 => 9217,  // ИС-6 Бесстрашный
        64801 => 2849,  // T34 Independence
        _ => tank_id,
    }
}

/// Updates the bundled `tankopedia.json` and generates the bundled [`phf::Map`] with the tankopedia.
#[tracing::instrument(skip_all)]
pub async fn import(opts: ImportTankopediaOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "import-tankopedia"));

    let api = WargamingApi::new(&opts.application_id)?;
    let json_path = Path::new(file!())
        .parent()
        .unwrap()
        .join("tankopedia")
        .join("tankopedia.json");
    let tankopedia: Tankopedia =
        serde_json::from_str::<Tankopedia>(&fs::read_to_string(&json_path)?)?
            .into_iter()
            .chain(api.get_tankopedia().await?)
            .collect();
    fs::write(&json_path, serde_json::to_string_pretty(&tankopedia)?)?;

    let mut vehicles: BTreeMap<String, Vehicle> =
        serde_json::from_value(serde_json::to_value(&tankopedia)?)?;
    insert_missing_vehicles(&mut vehicles)?;
    tracing::info!(n_vehicles = vehicles.len(), "finished");

    let mut file = fs::File::create(
        Path::new(file!())
            .parent()
            .unwrap()
            .join("tankopedia")
            .join("generated.rs"),
    )?;
    writeln!(&mut file, "//! @generated")?;
    writeln!(&mut file)?;
    writeln!(&mut file, "use std::borrow::Cow;")?;
    writeln!(&mut file)?;
    writeln!(
        &mut file,
        "use crate::models::{{Nation, TankType, Vehicle}};"
    )?;
    writeln!(&mut file)?;
    writeln!(
        &mut file,
        "pub static GENERATED: phf::Map<i32, Vehicle> = phf::phf_map! {{"
    )?;
    for (_, vehicle) in vehicles.into_iter() {
        writeln!(&mut file, "    {}_i32 => Vehicle {{", vehicle.tank_id)?;
        writeln!(&mut file, "        tank_id: {:?},", vehicle.tank_id)?;
        writeln!(
            &mut file,
            "        name: Cow::Borrowed({:?}),",
            vehicle.name,
        )?;
        writeln!(&mut file, "        tier: {:?},", vehicle.tier)?;
        writeln!(&mut file, "        is_premium: {:?},", vehicle.is_premium)?;
        writeln!(&mut file, "        nation: Nation::{:?},", vehicle.nation)?;
        writeln!(&mut file, "        type_: TankType::{:?},", vehicle.type_)?;
        writeln!(&mut file, "    }},")?;
    }
    writeln!(&mut file, "}};")?;

    Ok(())
}

/// Inserts the hand-coded tanks that are somehow missing from the Tankopedia.
fn insert_missing_vehicles(vehicles: &mut BTreeMap<String, Vehicle>) -> crate::Result {
    for vehicle in [
        Vehicle {
            tank_id: 23057,
            name: Cow::Borrowed("Kunze Panzer"),
            tier: 7,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 20817,
            name: Cow::Borrowed("Эксплорер"),
            tier: 6,
            is_premium: true,
            nation: Nation::Uk,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 3089,
            name: Cow::Borrowed("Leichttraktor"),
            tier: 1,
            is_premium: false,
            nation: Nation::Germany,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 64081,
            name: Cow::Borrowed("Mk I* Heavy Tank"),
            tier: 1,
            is_premium: true,
            nation: Nation::Uk,
            type_: TankType::Heavy,
        },
        Vehicle {
            tank_id: 1329,
            name: Cow::Borrowed("Renault NC-31"),
            tier: 1,
            is_premium: false,
            nation: Nation::China,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 609,
            name: Cow::Borrowed("R. Otsu"),
            tier: 1,
            is_premium: false,
            nation: Nation::Japan,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 3329,
            name: Cow::Borrowed("MS-1 mod. 1"),
            tier: 1,
            is_premium: false,
            nation: Nation::Ussr,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 24081,
            name: Cow::Borrowed("U-Panzer"),
            tier: 6,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 23825,
            name: Cow::Borrowed("Steyr WT"),
            tier: 7,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::AT,
        },
        Vehicle {
            tank_id: 23297,
            name: Cow::Borrowed("Объект 244"),
            tier: 6,
            is_premium: true,
            nation: Nation::Ussr,
            type_: TankType::Heavy,
        },
        Vehicle {
            tank_id: 18241,
            name: Cow::Borrowed("B-C Bourrasque"),
            tier: 8,
            is_premium: true,
            nation: Nation::France,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 577,
            name: Cow::Borrowed("Renault FT"),
            tier: 2,
            is_premium: true,
            nation: Nation::France,
            type_: TankType::AT,
        },
        Vehicle {
            tank_id: 81,
            name: Cow::Borrowed("Vickers Medium Mk. I"),
            tier: 1,
            is_premium: true,
            nation: Nation::Uk,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 545,
            name: Cow::Borrowed("T1 Cunningham"),
            tier: 1,
            is_premium: true,
            nation: Nation::Usa,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 23313,
            name: Cow::Borrowed("Kampfpanzer 50 t"),
            tier: 10,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 15617,
            name: Cow::Borrowed("Объект 907"),
            tier: 10,
            is_premium: true,
            nation: Nation::Ussr,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 22529,
            name: Cow::Borrowed("ЛТ-432"),
            tier: 8,
            is_premium: true,
            nation: Nation::Ussr,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 8577,
            name: Cow::Borrowed("Lansen C"),
            tier: 8,
            is_premium: true,
            nation: Nation::Europe,
            type_: TankType::Medium,
        },
        Vehicle {
            tank_id: 24321,
            name: Cow::Borrowed("Т-100 ЛТ"),
            tier: 10,
            is_premium: true,
            nation: Nation::Ussr,
            type_: TankType::Light,
        },
        Vehicle {
            tank_id: 24577,
            name: Cow::Borrowed("Объект 268/4"),
            tier: 10,
            is_premium: true,
            nation: Nation::Ussr,
            type_: TankType::AT,
        },
        Vehicle {
            tank_id: 9089,
            name: Cow::Borrowed("Škoda T 56"),
            tier: 8,
            is_premium: true,
            nation: Nation::Europe,
            type_: TankType::Heavy,
        },
        Vehicle {
            tank_id: 24593,
            name: Cow::Borrowed("Škoda T 56"),
            tier: 8,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::Heavy,
        },
    ] {
        match vehicles.get(&vehicle.tank_id.to_string()) {
            Some(_) => anyhow::bail!("vehicle #{} is already in the tankopedia", vehicle.tank_id),
            None => vehicles.insert(vehicle.tank_id.to_string(), vehicle),
        };
    }

    Ok(())
}
