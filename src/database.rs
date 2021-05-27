use std::borrow::Borrow;
use std::fmt::Debug;
use std::time::Instant;

use chrono::{Duration, SubsecRound, Utc};
use mongodb::bson::{doc, Document};
use mongodb::options::{FindOneOptions, InsertManyOptions, ReplaceOptions};
use mongodb::results::UpdateResult;
use mongodb::Collection;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::logging::log_anyhow;

pub mod models;

const DATABASE_NAME: &str = "blitz-dashboard";

/// Convenience collection container.
#[derive(Clone)]
pub struct Database {
    // FIXME: make these private, expose methods.
    pub accounts: Collection<models::Account>,
    pub account_snapshots: Collection<models::AccountSnapshot>,
    pub tank_snapshots: Collection<models::TankSnapshot>,
}

/// Used to derive an `upsert` query from a document.
pub trait UpsertQuery {
    fn query(&self) -> Document;
}

impl Database {
    const ACCOUNT_EXPIRATION_MINUTES: i64 = 5;

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

    /// Retrieve account given that it isn't older than the maximum battle duration.
    pub async fn get_account(&self, account_id: i32) -> crate::Result<Option<models::AccountInfo>> {
        let since =
            (Utc::now() - Duration::minutes(Self::ACCOUNT_EXPIRATION_MINUTES)).trunc_subsecs(3);
        log::debug!("get_account {} since {}", account_id, since);
        let account = self
            .accounts
            .find_one(
                doc! { "aid": account_id, "ts": { "$gt": since } },
                FindOneOptions::builder().show_record_id(false).build(),
            )
            .await?;
        if account.is_none() {
            return Ok(None);
        }
        let account_snapshot = self
            .account_snapshots
            .find_one(
                doc! { "aid": account_id },
                FindOneOptions::builder()
                    .show_record_id(false)
                    .sort(doc! { "lbts": -1 })
                    .build(),
            )
            .await?;
        if account_snapshot.is_none() {
            return Ok(None);
        }
        Ok(Some(models::AccountInfo(
            account.unwrap(),
            account_snapshot.unwrap(),
        )))
    }

    /// Saves the account statistics to the database.
    pub async fn save_snapshots(
        &self,
        account_info: impl Into<models::AccountInfo>,
        tanks_stats: Vec<crate::wargaming::models::TankStatistics>,
    ) {
        let models::AccountInfo(account, account_info) = account_info.into();
        let start = Instant::now();
        log_anyhow(upsert(&self.accounts, account).await);
        log_anyhow(upsert(&self.account_snapshots, account_info).await);
        let _ = self
            // Unfortunately, I have to ignore errors here,
            // because the driver doesn't support the proper bulk operations.
            .tank_snapshots
            .insert_many(
                tanks_stats
                    .into_iter()
                    .map(Into::<models::TankSnapshot>::into),
                InsertManyOptions::builder().ordered(false).build(),
            )
            .await;
        log::debug!("Snapshots saved in {:#?}.", Instant::now() - start);
    }
}

/// Convenience wrapper around `[mongodb::Collection::replace_one]`.
/// Automatically constructs a query with the `[UpsertQuery]` trait and sets the `upsert` flag.
pub async fn upsert<T, R>(collection: &Collection<T>, replacement: R) -> crate::Result<UpdateResult>
where
    T: Serialize + DeserializeOwned + Unpin + Debug + UpsertQuery,
    R: Borrow<T>,
{
    let replacement = replacement.borrow();
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
