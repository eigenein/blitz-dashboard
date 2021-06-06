use std::time::Instant;

use chrono::Utc;
use lazy_static::lazy_static;
use mongodb::bson::{doc, Bson, Document};
use mongodb::options::{FindOneOptions, ReplaceOptions};
use mongodb::results::UpdateResult;

pub mod models;

lazy_static! {
    static ref OPTIONS_UPSERT: Option<ReplaceOptions> =
        Some(ReplaceOptions::builder().upsert(true).build());
}

/// Convenience collection container.
#[derive(Clone)]
pub struct Database {
    database: mongodb::Database,
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
            .collection::<models::AccountUpdatedAt>(Self::ACCOUNT_COLLECTION)
            .find_one(
                doc! { "aid": account_id },
                FindOneOptions::builder()
                    .projection(doc! { "ts": 1 })
                    .build(),
            )
            .await?;
        Ok(document.map(|document| document.updated_at.into()))
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
        let query = doc! { "aid": account.id };
        Ok(self
            .database
            .collection::<models::Account>(Self::ACCOUNT_COLLECTION)
            .replace_one(query, account, OPTIONS_UPSERT.clone())
            .await?)
    }

    pub async fn upsert_account_snapshot(
        &self,
        account_snapshot: &models::AccountSnapshot,
    ) -> crate::Result<UpdateResult> {
        log::debug!(
            "Upserting account #{} snapshot…",
            account_snapshot.account_id
        );
        let query = doc! { "aid": account_snapshot.account_id, "lbts": Bson::DateTime(account_snapshot.last_battle_time) };
        Ok(self
            .database
            .collection::<models::AccountSnapshot>(Self::ACCOUNT_SNAPSHOT_COLLECTION)
            .replace_one(query, account_snapshot, OPTIONS_UPSERT.clone())
            .await?)
    }

    pub async fn upsert_tank_snapshot(
        &self,
        tank_snapshot: &models::TankSnapshot,
    ) -> crate::Result<UpdateResult> {
        let query = doc! {
            "aid": tank_snapshot.account_id,
            "tid": tank_snapshot.tank_id,
            "lbts": Bson::DateTime(tank_snapshot.last_battle_time),
        };
        Ok(self
            .database
            .collection::<models::TankSnapshot>(Self::TANK_SNAPSHOT_COLLECTION)
            .replace_one(query, tank_snapshot, OPTIONS_UPSERT.clone())
            .await?)
    }

    pub async fn upsert_full_info(
        &self,
        account_info: &crate::wargaming::models::AccountInfo,
        tanks: &Vec<(
            i32,
            (
                crate::wargaming::models::TankStatistics,
                crate::wargaming::models::TankAchievements,
            ),
        )>,
    ) -> crate::Result {
        log::debug!("Upserting account #{} info…", account_info.id);
        let start = Instant::now();
        let account_updated_at = self.get_account_updated_at(account_info.id).await?;
        self.upsert_account(&account_info.into()).await?;
        self.upsert_account_snapshot(&account_info.into()).await?;
        let mut selected_tank_count: i32 = 0;
        for (_, (stats, _achievements)) in tanks {
            // FIXME:
            if account_updated_at.is_none() || stats.last_battle_time >= account_updated_at.unwrap()
            {
                selected_tank_count += 1;
                // TODO: combine stats and achievements.
                self.upsert_tank_snapshot(&stats.into()).await?;
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
