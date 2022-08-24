use futures::{Stream, TryStreamExt};
use mongodb::bson::{doc, Document};
use mongodb::options::{FindOptions, IndexOptions};
use mongodb::{bson, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::database::mongodb::traits::{Indexes, TypedDocument, Upsert};
use crate::helpers::serde::is_default;
use crate::helpers::time::from_months;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TrainItem {
    #[serde(rename = "rlm")]
    pub realm: wargaming::Realm,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "tid")]
    pub tank_id: wargaming::TankId,

    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "nb")]
    pub n_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(default, rename = "nw", skip_serializing_if = "is_default")]
    pub n_wins: u32,
}

impl TypedDocument for TrainItem {
    const NAME: &'static str = "train";
}

impl Indexes for TrainItem {
    type I = [IndexModel; 2];

    fn indexes() -> Self::I {
        [
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "lbts": -1, "tid": 1, "aid": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "lbts": 1 })
                .options(IndexOptions::builder().expire_after(from_months(1)).build())
                .build(),
        ]
    }
}

impl Upsert for TrainItem {
    type Update = Document;

    fn query(&self) -> Document {
        doc! {
            "rlm": self.realm.to_str(),
            "lbts": self.last_battle_time,
            "aid": self.account_id,
            "tid": self.tank_id,
        }
    }

    fn update(&self) -> Result<Self::Update> {
        Ok(doc! { "$setOnInsert": bson::to_bson(&self)? })
    }
}

impl TrainItem {
    #[instrument(level = "info", skip_all, fields(realm = ?realm, since = ?since))]
    pub async fn stream(
        from: &Database,
        realm: wargaming::Realm,
        since: DateTime,
    ) -> Result<impl Stream<Item = Result<Self>>> {
        let filter = doc! {
            // TODO: "rlm": realm.to_str(),
            "lbts": { "$gte": since },
        };
        let options = FindOptions::builder().projection(doc! { "_id": 0 }).build();
        let stream = Self::collection(from)
            .find(filter, options)
            .await
            .context("failed to query train items")?
            .map_err(Error::from);
        Ok(stream)
    }
}