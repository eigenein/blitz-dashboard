use std::time::Instant;

use mongodb::bson::{doc, Document};
use mongodb::options::{UpdateModifications, UpdateOptions};
use mongodb::results::UpdateResult;
use mongodb::Database;
use serde::{Deserialize, Serialize};

use crate::format_elapsed;
use crate::prelude::*;
use crate::wargaming::models::BaseAccountInfo;

#[derive(Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "_id")]
    pub id: i32,

    #[serde(rename = "lbts")]
    pub last_battle_time: DateTime,
}

impl From<BaseAccountInfo> for Account {
    fn from(account_info: BaseAccountInfo) -> Self {
        Self {
            id: account_info.id,
            last_battle_time: account_info.last_battle_time,
        }
    }
}

impl Account {
    pub const COLLECTION_NAME: &'static str = "accounts";
    pub const LAST_BATTLE_TIME_FIELD_NAME: &'static str = "lbts";

    #[instrument(skip_all, level = "debug", fields(account_id = self.id, result))]
    pub async fn upsert(&self, to: &Database) -> Result<UpdateResult> {
        let start_instant = Instant::now();
        let result = to
            .collection::<Document>(Self::COLLECTION_NAME)
            .update_one(
                doc! { "_id": self.id },
                UpdateModifications::Document(
                    doc! { "$set": { Self::LAST_BATTLE_TIME_FIELD_NAME: self.last_battle_time } },
                ),
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .with_context(|| format!("failed to upsert the account #{}", self.id))?;
        debug!(
            account_id = self.id,
            elapsed = format_elapsed(start_instant).as_str(),
            "upserted",
        );
        Ok(result)
    }
}
