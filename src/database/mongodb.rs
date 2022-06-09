use mongodb::Database;

use crate::prelude::*;

pub mod models;

#[instrument(level = "debug")]
pub async fn open(uri: &str) -> Result<Database> {
    info!(uri, "connecting…");
    let client = mongodb::Client::with_uri_str(uri)
        .await
        .context("failed to parse the specified MongoDB URI")?;
    let database = client
        .default_database()
        .ok_or_else(|| anyhow!("MongoDB database name is not specified"))?;

    info!("ensuring indexes…");
    models::Account::ensure_indexes(&database).await?;
    models::TankSnapshot::ensure_indexes(&database).await?;

    info!("connected");
    Ok(database)
}
