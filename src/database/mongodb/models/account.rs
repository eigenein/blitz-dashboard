use futures::{Stream, TryStreamExt};
use mongodb::bson::doc;
use mongodb::options::UpdateOptions;
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::{format_elapsed, wargaming};

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "_id")]
    pub id: wargaming::AccountId,

    #[serde(rename = "lbts")]
    #[serde_as(as = "Option<bson::DateTime>")]
    pub last_battle_time: Option<DateTime>,
}

impl From<wargaming::AccountInfo> for Account {
    fn from(account_info: wargaming::AccountInfo) -> Self {
        Self {
            id: account_info.id,
            last_battle_time: Some(account_info.last_battle_time),
        }
    }
}

impl Account {
    pub const OPERATION_SET: &'static str = "$set";
    pub const OPERATION_SET_ON_INSERT: &'static str = "$setOnInsert";

    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("accounts")
    }

    pub fn fake(account_id: wargaming::AccountId) -> Self {
        Self {
            id: account_id,
            last_battle_time: None,
        }
    }

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
    ) -> Result<impl Stream<Item = Result<Account>>> {
        let now = Utc::now();
        let filter = doc! {
            "random": { "$gt": fastrand::f64() },
            "$or": [
                { "lbts": null },
                { "lbts": bson::DateTime::from_millis(0) }, // FIXME: remove.
                { "lbts": { "$gt": now - max_offset, "$lte": now - min_offset } },
            ],
        };

        let start_instant = Instant::now();
        debug!(sample_size, "retrieving a sample…");
        let account_stream = Self::collection(from)
            .find(filter, None)
            .instrument(debug_span!("aggregate"))
            .await
            .context("failed to query a sample of accounts")?
            .map_err(|error| anyhow!(error))
            .instrument(debug_span!("sampled_account"));

        debug!(elapsed = format_elapsed(start_instant).as_str(), "done");
        Ok(account_stream)
    }
}
