use anyhow::Error;
use futures::stream::{iter, try_unfold};
use futures::{Stream, TryStreamExt};
use mongodb::bson::doc;
use mongodb::options::{FindOptions, UpdateOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::prelude::*;
use crate::{format_elapsed, wargaming};

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Account {
    #[serde(rename = "_id")]
    pub id: wargaming::AccountId,

    #[serde(rename = "lbts")]
    #[serde_as(as = "Option<bson::DateTime>")]
    pub last_battle_time: Option<DateTime>,
}

impl Account {
    pub fn new(id: wargaming::AccountId, last_battle_time: DateTime) -> Self {
        Self {
            id,
            last_battle_time: Some(last_battle_time),
        }
    }

    pub fn fake(account_id: wargaming::AccountId) -> Self {
        Self {
            id: account_id,
            last_battle_time: None,
        }
    }
}

impl Account {
    pub const OPERATION_SET: &'static str = "$set";
    pub const OPERATION_SET_ON_INSERT: &'static str = "$setOnInsert";

    #[instrument(skip_all)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        let indexes = [
            IndexModel::builder().keys(doc! { "lbts": -1 }).build(),
            IndexModel::builder().keys(doc! { "random": 1 }).build(),
        ];
        Self::collection(on)
            .create_indexes(indexes, None)
            .await
            .context("failed to create the indexes on accounts")?;
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    pub fn get_sampled_stream(
        database: Database,
        sample_size: u32,
        min_offset: Duration,
        max_offset: Duration,
    ) -> impl Stream<Item = Result<Self>> {
        info!(sample_size, %min_offset, %max_offset);
        try_unfold((1, database), move |(sample_number, database)| async move {
            debug!(sample_number, "retrieving a sample…");
            let future = Account::retrieve_sample(&database, sample_size, min_offset, max_offset);
            let sample = timeout(StdDuration::from_secs(60), future) // FIXME.
                .await
                .with_context(|| format!("timed out to retrieve sample #{}", sample_number))??;
            debug!(sample_number, "retrieved");
            Ok::<_, Error>(Some((iter(sample.into_iter().map(Ok)), (sample_number + 1, database))))
        })
        .try_flatten()
    }

    #[instrument(skip_all, level = "debug", fields(account_id = self.id, operation = operation))]
    pub async fn upsert(&self, to: &Database, operation: &str) -> Result {
        let query = doc! { "_id": self.id };
        let update =
            doc! { operation: { "lbts": self.last_battle_time, "random": fastrand::f64() } };
        let options = UpdateOptions::builder().upsert(true).build();

        debug!("upserting…");
        let start_instant = Instant::now();
        Self::collection(to)
            .update_one(query, update, options)
            .await
            .with_context(|| format!("failed to upsert the account #{}", self.id))?;

        debug!(elapsed = format_elapsed(start_instant).as_str(), "upserted");
        Ok(())
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn retrieve_sample(
        from: &Database,
        sample_size: u32,
        min_offset: Duration,
        max_offset: Duration,
    ) -> Result<Vec<Account>> {
        let now = Utc::now();
        let filter = doc! {
            "random": { "$gt": fastrand::f64() },
            "$or": [
                { "lbts": null },
                { "lbts": bson::DateTime::from_millis(0) }, // FIXME: remove.
                { "lbts": { "$gt": now - max_offset, "$lte": now - min_offset } },
            ],
        };
        let options = FindOptions::builder()
            .sort(doc! { "random": 1 })
            .limit(sample_size as i64)
            .build();

        let start_instant = Instant::now();
        debug!(sample_size, "retrieving a sample…");
        let accounts: Vec<Account> = Self::collection(from)
            .find(filter, options)
            .await
            .context("failed to query a sample of accounts")?
            .try_collect()
            .await?;

        debug!(
            n_accounts = accounts.len(),
            elapsed = format_elapsed(start_instant).as_str(),
            "done",
        );
        Ok(accounts)
    }

    // TODO: retrieve one random account and implement «I'm feeling lucky».
}

impl Account {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("accounts")
    }
}
