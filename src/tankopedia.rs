use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration as StdDuration;

use crate::models::{Nation, TankType, Vehicle};
use crate::opts::ImportTankopediaOpts;
use crate::wargaming::{Tankopedia, WargamingApi};

mod generated;

pub fn get_vehicle(tank_id: i32) -> Vehicle {
    generated::GENERATED
        .get(&tank_id)
        .cloned() // FIXME: avoid `cloned()`.
        .unwrap_or_else(|| new_hardcoded_vehicle(tank_id))
}

fn new_hardcoded_vehicle(tank_id: i32) -> Vehicle {
    Vehicle {
        tank_id,
        name: Cow::Owned(format!("#{}", tank_id)),
        tier: 0,
        is_premium: false,
        type_: TankType::Unknown,
        nation: Nation::Other,
    }
}

/// Updates the bundled `tankopedia.json` and generates the bundled [`phf::Map`] with the tankopedia.
pub async fn import(opts: ImportTankopediaOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "import-tankopedia"));

    let api = WargamingApi::new(&opts.application_id, StdDuration::from_millis(5000))?;
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

    let mut file = fs::File::create(
        Path::new(file!())
            .parent()
            .unwrap()
            .join("tankopedia")
            .join("generated.rs"),
    )?;
    writeln!(&mut file, "//! Generated tankopedia.")?;
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

fn insert_missing_vehicles(vehicles: &mut BTreeMap<String, Vehicle>) -> crate::Result {
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 23057,
            name: Cow::Borrowed("Kunze Panzer"),
            tier: 7,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::Light,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 20817,
            name: Cow::Borrowed("Эксплорер"),
            tier: 6,
            is_premium: true,
            nation: Nation::Uk,
            type_: TankType::Medium,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 3089,
            name: Cow::Borrowed("Leichttraktor"),
            tier: 1,
            is_premium: false,
            nation: Nation::Germany,
            type_: TankType::Light,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 64081,
            name: Cow::Borrowed("Mk I* Heavy Tank"),
            tier: 1,
            is_premium: true,
            nation: Nation::Uk,
            type_: TankType::Heavy,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 1329,
            name: Cow::Borrowed("Renault NC-31"),
            tier: 1,
            is_premium: false,
            nation: Nation::China,
            type_: TankType::Light,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 609,
            name: Cow::Borrowed("R. Otsu"),
            tier: 1,
            is_premium: false,
            nation: Nation::Japan,
            type_: TankType::Light,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 3329,
            name: Cow::Borrowed("MS-1"),
            tier: 1,
            is_premium: false,
            nation: Nation::Ussr,
            type_: TankType::Light,
        },
    )?;
    insert_missing_vehicle(
        vehicles,
        Vehicle {
            tank_id: 24081,
            name: Cow::Borrowed("U-Panzer"),
            tier: 6,
            is_premium: true,
            nation: Nation::Germany,
            type_: TankType::Medium,
        },
    )?;
    Ok(())
}

fn insert_missing_vehicle(
    vehicles: &mut BTreeMap<String, Vehicle>,
    vehicle: Vehicle,
) -> crate::Result {
    match vehicles.get(&vehicle.tank_id.to_string()) {
        Some(_) => anyhow::bail!("vehicle #{} is already in the tankopedia", vehicle.tank_id),
        None => vehicles.insert(vehicle.tank_id.to_string(), vehicle),
    };
    Ok(())
}
