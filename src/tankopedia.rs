use std::time::Instant;

use crate::database::Database;
use crate::models::Vehicle;
use crate::wargaming::WargamingApi;

pub async fn run(api: WargamingApi, database: Database) -> crate::Result {
    log::info!("Starting the importer…");
    let start_instant = Instant::now();
    database.upsert_vehicles(
        &api.get_tankopedia()
            .await?
            .into_iter()
            .map(|(_, vehicle)| vehicle)
            .collect::<Vec<Vehicle>>(),
    )?;
    log::info!("All done in {:?}.", Instant::now() - start_instant);
    Ok(())
}
