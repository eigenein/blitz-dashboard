use mongodb::bson::{doc, Document};
use mongodb::options::{FindOneOptions, IndexOptions};
use mongodb::{bson, Database, IndexModel};
use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;

use crate::database::mongodb::traits::{TypedDocument, Upsert};
use crate::database::{RandomStatsSnapshot, RatingStatsSnapshot, TankLastBattleTime};
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct AccountSnapshot {
    #[serde(rename = "rlm")]
    pub realm: wargaming::Realm,

    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde(flatten)]
    pub random_stats: RandomStatsSnapshot,

    #[serde(flatten)]
    pub rating_stats: RatingStatsSnapshot,

    #[serde(rename = "t")]
    pub tank_last_battle_times: Vec<TankLastBattleTime>,
}

impl TypedDocument for AccountSnapshot {
    const NAME: &'static str = "account_snapshots";
}

impl AccountSnapshot {
    pub fn new(
        realm: wargaming::Realm,
        account_info: &wargaming::AccountInfo,
        tank_last_battle_times: Vec<TankLastBattleTime>,
    ) -> Self {
        Self {
            realm,
            last_battle_time: account_info.last_battle_time,
            account_id: account_info.id,
            random_stats: account_info.stats.random.into(),
            rating_stats: account_info.stats.rating.into(),
            tank_last_battle_times,
        }
    }
}

#[async_trait]
impl Upsert for AccountSnapshot {
    type Update = Document;

    #[inline]
    fn query(&self) -> Document {
        doc! {
            "rlm": self.realm.to_str(),
            "aid": self.account_id,
            "lbts": self.last_battle_time,
        }
    }

    #[inline]
    fn update(&self) -> Result<Self::Update> {
        Ok(doc! { "$setOnInsert": bson::to_bson(self)? })
    }
}

impl AccountSnapshot {
    #[instrument(skip_all, err)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        let indexes = [IndexModel::builder()
            .keys(doc! { "rlm": 1, "aid": 1, "lbts": -1 })
            .options(IndexOptions::builder().unique(true).build())
            .build()];
        Self::collection(on)
            .create_indexes(indexes, None)
            .await
            .context("failed to create the indexes on account snapshots")?;
        Ok(())
    }

    #[instrument(skip_all, fields(account_id = account_id, before = ?before), err)]
    pub async fn retrieve_latest(
        from: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        before: DateTime,
    ) -> Result<Option<Self>> {
        let filter = doc! { "rlm": realm.to_str(), "aid": account_id, "lbts": { "$lte": before } };
        let options = FindOneOptions::builder().sort(doc! { "lbts": -1 }).build();
        let start_instant = Instant::now();
        let this = Self::collection(from).find_one(filter, options).await?;
        if let Some(this) = &this {
            debug!(?this.last_battle_time, "found");
        }
        debug!(elapsed_secs = start_instant.elapsed().as_secs_f32());
        Ok(this)
    }
}
