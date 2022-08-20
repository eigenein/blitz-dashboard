mod vehicle_stats;

use futures::{Stream, TryStreamExt};
use mongodb::bson::{doc, Document};
use mongodb::options::IndexOptions;
use mongodb::{bson, IndexModel};
use serde::{Deserialize, Serialize};

pub use self::vehicle_stats::*;
use crate::database::mongodb::traits::{Indexes, TypedDocument, Upsert};
use crate::database::TankSnapshot;
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
    #[serde(default, rename = "nb", skip_serializing_if = "is_default")]
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
            // 1. Optimizes search queries by realm & last battle time.
            // 2. Ensures that the upserts work correctly.
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "lbts": -1, "aid": 1, "tid": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            // Ensures expiration of the items.
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
        Ok(doc! { "$set": bson::to_bson(&self)? })
    }
}

impl TrainItem {
    pub const fn new(
        actual_snapshot: &TankSnapshot,
        previous_snapshot: &TankSnapshot,
    ) -> Option<Self> {
        if actual_snapshot.stats.n_battles > previous_snapshot.stats.n_battles {
            Some(Self {
                realm: actual_snapshot.realm,
                account_id: actual_snapshot.account_id,
                tank_id: actual_snapshot.tank_id,
                last_battle_time: actual_snapshot.last_battle_time,
                n_wins: actual_snapshot.stats.n_wins - previous_snapshot.stats.n_wins,
                n_battles: actual_snapshot.stats.n_battles - previous_snapshot.stats.n_battles,
            })
        } else {
            None
        }
    }

    #[instrument(skip_all, fields(after = ?after))]
    pub async fn retrieve_all(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        after: DateTime,
    ) -> Result<impl Stream<Item = Result<Self>>> {
        let filter = doc! { "rlm": realm.to_str(), "lbts": { "$gte": after } };
        let stream = Self::collection(from)
            .find(filter, None)
            .await
            .context("failed to query train items")?
            .map_err(Error::from);
        Ok(stream)
    }
}
