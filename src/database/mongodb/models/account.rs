use futures::future::ready;
use futures::{Stream, TryStreamExt};
use mongodb::bson::{doc, from_document};
use mongodb::options::{UpdateModifications, UpdateOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::{format_elapsed, wargaming};

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "_id")]
    pub id: i32,

    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,
}

impl From<wargaming::AccountInfo> for Account {
    fn from(account_info: wargaming::AccountInfo) -> Self {
        Self {
            id: account_info.id,
            last_battle_time: account_info.last_battle_time,
        }
    }
}

impl Account {
    const COLLECTION_NAME: &'static str = "accounts";
    const LAST_BATTLE_TIME_KEY: &'static str = "lbts";
    pub const OPERATION_SET: &'static str = "$set";
    pub const OPERATION_SET_ON_INSERT: &'static str = "$setOnInsert";

    pub fn fake(account_id: i32) -> Self {
        Self {
            id: account_id,
            last_battle_time: Utc.timestamp(0, 0),
        }
    }

    #[instrument(skip_all)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        Self::collection(on)
            .create_index(
                IndexModel::builder()
                    .keys(doc! { Self::LAST_BATTLE_TIME_KEY: -1 })
                    .build(),
                None,
            )
            .await
            .context("failed to create `accounts.lbts` index")?;
        Ok(())
    }

    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection(Self::COLLECTION_NAME)
    }

    #[instrument(skip_all, level = "debug", fields(account_id = self.id, operation = operation))]
    pub async fn upsert(&self, to: &Database, operation: &str) -> Result {
        let query = doc! { "_id": self.id };
        let update = UpdateModifications::Document(
            doc! { operation: { Self::LAST_BATTLE_TIME_KEY: self.last_battle_time } },
        );
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
        let pipeline = [
            doc! { "$sample": { "size": sample_size } },
            doc! {
                "$match": {
                    "$or": [
                        { Self::LAST_BATTLE_TIME_KEY: null },
                        { Self::LAST_BATTLE_TIME_KEY: bson::DateTime::from_millis(0) }, // FIXME: remove.
                        { Self::LAST_BATTLE_TIME_KEY: { "$gt": now - max_offset, "$lte": now - min_offset } },
                    ],
                },
            },
        ];

        let start_instant = Instant::now();
        debug!(sample_size, "retrieving a sample…");
        let account_stream = Self::collection(from)
            .aggregate(pipeline, None)
            .instrument(debug_span!("aggregate"))
            .await
            .context("failed to query a sample of accounts")?
            .map_err(|error| anyhow!(error))
            .try_filter_map(|document| {
                trace!(?document);
                ready(
                    from_document::<Account>(document)
                        .map(Some)
                        .map_err(|error| anyhow!("failed to deserialize an account: {}", error)),
                )
            })
            .instrument(debug_span!("sampled_account"));

        debug!(elapsed = format_elapsed(start_instant).as_str(), "done");
        Ok(account_stream)
    }
}