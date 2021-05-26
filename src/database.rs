pub mod models;

use mongodb::bson::{doc, Document};
use mongodb::options::ReplaceOptions;
use mongodb::results::UpdateResult;
use mongodb::Collection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

const DATABASE_NAME: &str = "blitz-dashboard";

/// Convenience collection container.
#[derive(Clone)]
pub struct Database {
    pub accounts: Collection<models::Account>,
    pub account_snapshots: Collection<models::AccountSnapshot>,
    pub tank_snapshots: Collection<models::TankSnapshot>,
}

/// Used to derive an `upsert` query from a document.
pub trait UpsertQuery {
    fn query(&self) -> Document;
}

impl Database {
    /// Open and initialize the database.
    pub async fn with_uri_str(uri: &str) -> crate::Result<Self> {
        log::info!("Connecting to the database…");
        let client = mongodb::Client::with_uri_str(uri).await?;
        let database = client.database(DATABASE_NAME);

        log::info!("Initializing the database…");
        create_index(&database, "accounts", doc! {"aid": 1}, "aid").await?;
        create_index(
            &database,
            "account_snapshots",
            doc! {"aid": 1, "lbts": -1},
            "aid_lbts",
        )
        .await?;
        create_index(
            &database,
            "tank_snapshots",
            doc! {"aid": 1, "tid": 1, "lbts": -1},
            "aid_tid_lbts",
        )
        .await?;

        Ok(Database {
            accounts: database.collection("accounts"),
            account_snapshots: database.collection("account_snapshots"),
            tank_snapshots: database.collection("tank_snapshots"),
        })
    }
}

/// Convenience wrapper around `[mongodb::Collection::replace_one]`.
/// Automatically constructs a query with the `[UpsertQuery]` trait and sets the `upsert` flag.
pub async fn upsert<T, R>(collection: &Collection<T>, replacement: R) -> crate::Result<UpdateResult>
where
    T: Serialize + DeserializeOwned + Unpin + Debug + UpsertQuery,
    R: Into<T>,
{
    let replacement = replacement.into();
    let query = replacement.query();
    let options = Some(ReplaceOptions::builder().upsert(true).build());
    Ok(collection.replace_one(query, replacement, options).await?)
}

async fn create_index(
    database: &mongodb::Database,
    collection: &str,
    key: Document,
    name: &str,
) -> crate::Result {
    let command = mongodb::bson::doc! {
        "createIndexes": collection,
        "indexes": [{"key": key, "name": name, "unique": true}],
    };
    database.run_command(command, None).await?;
    Ok(())
}
