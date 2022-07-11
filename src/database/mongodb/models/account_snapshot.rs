use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, IndexOptions, UpdateOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::database::{RandomStatsSnapshot, RatingStatsSnapshot, TankLastBattleTime};
use crate::helpers::tracing::format_elapsed;
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

    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde(flatten)]
    pub random_stats: RandomStatsSnapshot,

    #[serde(flatten)]
    pub rating_stats: RatingStatsSnapshot,

    #[serde(rename = "t")]
    pub tank_last_battle_times: Vec<TankLastBattleTime>,
}

impl AccountSnapshot {
    pub fn new(
        realm: wargaming::Realm,
        account_info: wargaming::AccountInfo,
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

impl AccountSnapshot {
    #[instrument(skip_all)]
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

    #[instrument(
        skip_all,
        level = "debug",
        fields(account_id = self.account_id),
    )]
    pub async fn upsert(&self, to: &Database) -> Result {
        let query = doc! { "rlm": self.realm.to_str(), "aid": self.account_id, "lbts": self.last_battle_time };
        let update = doc! { "$setOnInsert": bson::to_bson(self)? };
        let options = UpdateOptions::builder().upsert(true).build();

        debug!("upserting…");
        let start_instant = Instant::now();
        Self::collection(to)
            .update_one(query, update, options)
            .await
            .context("failed to upsert the account snapshot")?;

        debug!(elapsed = format_elapsed(start_instant).as_str(), "upserted");
        Ok(())
    }

    #[instrument(skip_all, level = "debug", fields(account_id = account_id, before = ?before))]
    pub async fn retrieve_latest(
        from: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        before: DateTime,
    ) -> Result<Option<Self>> {
        let filter = doc! { "rlm": realm.to_str(), "aid": account_id, "lbts": { "$lte": before } };
        let options = FindOptions::builder()
            .sort(doc! { "lbts": -1 })
            .limit(1)
            .build();
        let start_instant = Instant::now();
        let this = Self::collection(from)
            .find(filter, options)
            .await?
            .try_next()
            .await?;
        if let Some(this) = &this {
            debug!(?this.last_battle_time, "found");
        }
        debug!(elapsed_secs = start_instant.elapsed().as_secs_f32());
        Ok(this)
    }
}

impl AccountSnapshot {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("account_snapshots")
    }
}
