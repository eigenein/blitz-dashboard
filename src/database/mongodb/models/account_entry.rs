use futures::Stream;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::{Collection, Database};
use serde::Deserialize;

use crate::prelude::*;
use crate::wargaming;

#[derive(Deserialize, Copy, Clone)]
pub struct AccountEntry {
    #[serde(rename = "aid")]
    pub id: wargaming::AccountId,
}

impl AccountEntry {
    pub async fn retrieve_page(
        from: &Database,
        realm: &str,
        limit: i64,
    ) -> Result<impl Stream<Item = Result<Self, mongodb::error::Error>>> {
        let filter = doc! { "rlm": realm };
        let options = FindOptions::builder()
            .limit(limit)
            .sort(doc! { "rlm": 1, "lbts": -1 })
            .projection(doc! { "_id": 0, "aid": 1 })
            .build();
        Ok(Self::collection(from).find(filter, options).await?)
    }
}

impl AccountEntry {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("accounts")
    }
}
