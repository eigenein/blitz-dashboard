use log::Level;

use crate::database::Database;
use crate::metrics::Stopwatch;
use crate::models::Vehicle;
use crate::wargaming::WargamingApi;

pub async fn run(api: WargamingApi, database: Database) -> crate::Result {
    log::info!("Starting the importer…");
    let _stopwatch = Stopwatch::new("Imported").level(Level::Info);
    database.upsert_vehicles(
        &api.get_tankopedia()
            .await?
            .into_iter()
            .map(|(_, vehicle)| vehicle)
            .collect::<Vec<Vehicle>>(),
    )?;
    Ok(())
}
