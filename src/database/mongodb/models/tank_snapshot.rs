use std::collections::HashMap;

use futures::TryStreamExt;
use mongodb::bson::{doc, from_document};
use mongodb::options::{IndexOptions, UpdateModifications, UpdateOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::database::root::Root;
use crate::database::statistics_snapshot::StatisticsSnapshot;
use crate::helpers::tracing::format_elapsed;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct TankSnapshot {
    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,

    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde(rename = "tid")]
    pub tank_id: u32,

    #[serde(flatten)]
    pub statistics: StatisticsSnapshot,
}

impl From<wargaming::Tank> for TankSnapshot {
    fn from(tank: wargaming::Tank) -> Self {
        Self {
            last_battle_time: tank.statistics.basic.last_battle_time,
            account_id: tank.account_id,
            tank_id: tank.statistics.basic.tank_id as u32,
            statistics: tank.statistics.into(),
        }
    }
}

impl TankSnapshot {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("tank_snapshots")
    }

    #[instrument(skip_all)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        let indexes = [
            IndexModel::builder()
                .keys(doc! { "aid": 1, "tid": 1, "lbts": -1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "aid": 1, "lbts": -1 })
                .options(IndexOptions::builder().build())
                .build(),
        ];
        Self::collection(on)
            .create_indexes(indexes, None)
            .await
            .context("failed to create the indexes on tank snapshots")?;
        Ok(())
    }

    #[instrument(
        skip_all,
        level = "debug",
        fields(account_id = self.account_id, tank_id = self.tank_id),
    )]
    pub async fn upsert(&self, to: &Database) -> Result {
        let query = doc! {
            "aid": self.account_id,
            "tid": self.tank_id,
            "lbts": self.last_battle_time,
        };
        let update = UpdateModifications::Document(doc! { "$setOnInsert": bson::to_bson(self)? });
        let options = UpdateOptions::builder().upsert(true).build();

        debug!("upserting…");
        let start_instant = Instant::now();
        Self::collection(to)
            .update_one(query, update, options)
            .await
            .context("failed to upsert the tank snapshot")?;

        debug!(elapsed = format_elapsed(start_instant).as_str(), "upserted");
        Ok(())
    }

    #[instrument(
        skip_all,
        level = "debug",
        fields(account_id = account_id, before = ?before, n_tanks = tank_ids.len()),
    )]
    pub async fn retrieve_latest_tank_snapshots(
        from: &Database,
        account_id: wargaming::AccountId,
        before: DateTime,
        tank_ids: &[wargaming::TankId],
    ) -> Result<HashMap<wargaming::TankId, Self>> {
        if tank_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let pipeline = [
            doc! {
                "$match": {
                    "aid": account_id,
                    "tid": {"$in": tank_ids},
                    "lbts": {"$lt": before},
                },
            },
            doc! { "$sort": { "lbts": -1_i32 } },
            doc! { "$group": { "_id": "$tid", "root": { "$first": "$$ROOT" } } },
        ];

        let start_instant = Instant::now();
        debug!("running the pipeline…");
        let stream = Self::collection(from)
            .aggregate(pipeline, None)
            .await
            .with_context(|| {
                format!("failed to retrieve the latest tank snapshots for #{}", account_id)
            })?
            .map_err(|error| anyhow!(error))
            .try_filter_map(|document| async move {
                trace!(?document);
                let document = from_document::<Root<Self>>(document)?;
                Ok(Some((document.root.tank_id, document.root)))
            })
            .try_collect::<HashMap<wargaming::TankId, Self>>()
            .await?;

        debug!(elapsed = format_elapsed(start_instant).as_str(), "done");
        Ok(stream)
    }
}
