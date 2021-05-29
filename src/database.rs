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
        create_index(&database, Self::ACCOUNT_COLLECTION, doc! {"ts": 1}, "ts").await?;
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

    pub async fn get_oldest_account(&self) -> crate::Result<Option<models::Account>> {
        log::info!("Retrieving the oldest account…");
        Ok(self
            .database
            .collection(Self::ACCOUNT_COLLECTION)
            .find_one(
                None,
                FindOneOptions::builder()
                    .show_record_id(false)
                    .sort(doc! { "ts": 1 })
                    .build(),
            )
            .await?)
    }

    pub async fn upsert_account(&self, account: &models::Account) -> crate::Result<UpdateResult> {
        log::debug!("Upserting account #{}…", account.id);
        Self::upsert(&self.database.collection(Self::ACCOUNT_COLLECTION), account).await
    }

    pub async fn upsert_account_snapshot(
        &self,
        account_snapshot: &models::AccountSnapshot,
    ) -> crate::Result<UpdateResult> {
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

    pub async fn upsert_tank_snapshot(
        &self,
        tank_snapshot: &models::TankSnapshot,
    ) -> crate::Result<UpdateResult> {
        Self::upsert(
            &self.database.collection(Self::TANK_SNAPSHOT_COLLECTION),
            tank_snapshot,
        )
        .await
    }

    pub async fn upsert_account_info(
        &self,
        account_info: &crate::wargaming::models::AccountInfo,
        tanks_stats: &[crate::wargaming::models::TankStatistics],
    ) -> crate::Result {
        log::debug!("Upserting account #{} info…", account_info.id);
        let start = Instant::now();
        let account_updated_at = self.get_account_updated_at(account_info.id).await?;
        self.upsert_account(&account_info.into()).await?;
        self.upsert_account_snapshot(&account_info.into()).await?;
        let mut selected_tank_count = 0;
        for tank in tanks_stats {
            if account_updated_at.is_none() || tank.last_battle_time >= account_updated_at.unwrap()
            {
                selected_tank_count += 1;
                self.upsert_tank_snapshot(&tank.into()).await?;
            }
        }
        log::info!(
            "Account #{} info upserted in {:#?}. ({} tanks)",
            account_info.id,
            Instant::now() - start,
            selected_tank_count,
        );
        Ok(())
    }

    /// Convenience wrapper around `[mongodb::Collection::replace_one]`.
    /// Automatically constructs a query with the `[UpsertQuery]` trait and sets the `upsert` flag.
    async fn upsert<T>(collection: &Collection<T>, replacement: &T) -> crate::Result<UpdateResult>
    where
        T: Serialize + DeserializeOwned + Unpin + Debug + UpsertQuery,
    {
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
