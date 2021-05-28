use std::borrow::Borrow;
use std::fmt::Debug;
use std::time::Instant;

use chrono::Utc;
use mongodb::bson::{doc, Document};
use mongodb::options::{FindOneOptions, ReplaceOptions};
use mongodb::results::UpdateResult;
use mongodb::Collection;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod models;

/// Convenience collection container.
#[derive(Clone)]
pub struct Database {
    database: mongodb::Database,
}

/// Used to derive an `upsert` query from a document.
pub trait UpsertQuery {
    fn query(&self) -> Document;
}

impl Database {
    const DATABASE_NAME: &'static str = "blitz-dashboard";
    const ACCOUNT_SNAPSHOT_COLLECTION: &'static str = "account_snapshots";
    const ACCOUNT_COLLECTION: &'static str = "accounts";
    const TANK_SNAPSHOT_COLLECTION: &'static str = "tank_snapshots";

    /// Open and initialize the database.
    pub async fn with_uri_str(uri: &str) -> crate::Result<Self> {
        log::info!("Connecting to the database…");
        let client = mongodb::Client::with_uri_str(uri).await?;
        let database = client.database(Self::DATABASE_NAME);

        log::info!("Initializing the database…");
        create_index(&database, Self::ACCOUNT_COLLECTION, doc! {"aid": 1}, "aid").await?;
        create_index(
            &database,
            Self::ACCOUNT_SNAPSHOT_COLLECTION,
            doc! {"aid": 1, "lbts": -1},
            "aid_lbts",
        )
        .await?;
        create_index(
            &database,
            Self::TANK_SNAPSHOT_COLLECTION,
            doc! {"aid": 1, "tid": 1, "lbts": -1},
            "aid_tid_lbts",
        )
        .await?;

        Ok(Database { database })
    }

    pub async fn get_account_updated_at(
        &self,
        account_id: i32,
    ) -> crate::Result<Option<chrono::DateTime<Utc>>> {
        log::debug!("Retrieving account updated timestamp #{}…", account_id);
        let document = self
            .database
            .collection::<Document>(Self::ACCOUNT_COLLECTION)
            .find_one(
                doc! { "aid": account_id },
                FindOneOptions::builder()
                    .projection(doc! { "ts": 1 })
                    .build(),
            )
            .await?;
        match document {
            Some(document) => Ok(Some(document.get_datetime("ts")?.clone().into())),
            None => Ok(None),
        }
    }

    pub async fn upsert_account<A: Into<models::Account>>(
        &self,
        account: A,
    ) -> crate::Result<UpdateResult> {
        let account = account.into();
        log::debug!("Upserting account #{}…", account.id);
        Self::upsert(&self.database.collection(Self::ACCOUNT_COLLECTION), account).await
    }

    pub async fn upsert_account_snapshot<A: Into<models::AccountSnapshot>>(
        &self,
        account_snapshot: A,
    ) -> crate::Result<UpdateResult> {
        let account_snapshot = account_snapshot.into();
        log::debug!(
            "Upserting account #{} snapshot…",
            account_snapshot.account_id
        );
        Self::upsert(
            &self.database.collection(Self::ACCOUNT_SNAPSHOT_COLLECTION),
            account_snapshot,
        )
        .await
    }

    pub async fn upsert_tank_snapshot<T: Into<models::TankSnapshot>>(
        &self,
        tank_snapshot: T,
    ) -> crate::Result<UpdateResult> {
        Self::upsert(
            &self.database.collection(Self::TANK_SNAPSHOT_COLLECTION),
            tank_snapshot.into(),
        )
        .await
    }

    pub async fn upsert_account_info<A, S, T, TS>(
        &self,
        account: A,
        account_snapshot: S,
        tank_snapshots: TS,
    ) -> crate::Result
    where
        A: Into<models::Account>,
        S: Into<models::AccountSnapshot>,
        T: Into<models::TankSnapshot>,
        TS: Iterator<Item = T>,
    {
        let start = Instant::now();
        let account = account.into();
        let account_id = account.id;
        self.upsert_account(account).await?;
        self.upsert_account_snapshot(account_snapshot).await?;
        for tank_snapshot in tank_snapshots {
            self.upsert_tank_snapshot(tank_snapshot).await?;
        }
        log::info!(
            "Account #{} info upserted in {:#?}.",
            account_id,
            Instant::now() - start
        );
        Ok(())
    }

    /// Convenience wrapper around `[mongodb::Collection::replace_one]`.
    /// Automatically constructs a query with the `[UpsertQuery]` trait and sets the `upsert` flag.
    async fn upsert<T, R>(collection: &Collection<T>, replacement: R) -> crate::Result<UpdateResult>
    where
        T: Serialize + DeserializeOwned + Unpin + Debug + UpsertQuery,
        R: Borrow<T>,
    {
        let replacement = replacement.borrow();
        let query = replacement.query();
        let options = Some(ReplaceOptions::builder().upsert(true).build());
        Ok(collection.replace_one(query, replacement, options).await?)
    }
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
