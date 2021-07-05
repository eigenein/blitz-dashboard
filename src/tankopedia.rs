use log::Level;
use sqlx::PgPool;

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::Vehicle;
use crate::wargaming::WargamingApi;

pub async fn run(api: WargamingApi, database: PgPool) -> crate::Result {
    log::info!("Starting the importer…");
    let _stopwatch = Stopwatch::new("Imported").level(Level::Info);
    let mut transaction = database.begin().await?;
    database::insert_vehicles(
        &mut transaction,
        &api.get_tankopedia()
            .await?
            .into_iter()
            .map(|(_, vehicle)| vehicle)
            .collect::<Vec<Vehicle>>(),
    )
    .await?;
    log::info!("Committing…");
    transaction.commit().await?;
    Ok(())
}
