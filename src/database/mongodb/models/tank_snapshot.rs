use std::ops::Sub;

use futures::TryStreamExt;
use itertools::{merge_join_by, EitherOrBoth, Itertools};
use mongodb::bson::{doc, from_document, Document};
use mongodb::options::IndexOptions;
use mongodb::{bson, Database, IndexModel};
use serde::{Deserialize, Serialize};
use serde_with::TryFromInto;
use tokio::spawn;
use tokio::time::timeout;

use crate::database::mongodb::traits::{Indexes, TypedDocument, Upsert};
use crate::database::{RandomStatsSnapshot, Root, TankLastBattleTime};
use crate::helpers::tracing::format_elapsed;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct TankSnapshot {
    #[serde(rename = "rlm")]
    pub realm: wargaming::Realm,

    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "tid")]
    pub tank_id: wargaming::TankId,

    #[serde(rename = "life")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub battle_life_time: Duration,

    #[serde(flatten)]
    pub stats: RandomStatsSnapshot,
}

impl TypedDocument for TankSnapshot {
    const NAME: &'static str = "tank_snapshots";
}

impl Indexes for TankSnapshot {
    type I = [IndexModel; 2];

    fn indexes() -> Self::I {
        [
            // Ensures the only entry for each tank & last battle time.
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "aid": 1, "tid": 1, "lbts": -1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            // Optimizes the last battle time range queries for an account.
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "aid": 1, "lbts": -1 })
                .options(IndexOptions::builder().build())
                .build(),
        ]
    }
}

#[async_trait]
impl Upsert for TankSnapshot {
    type Update = Document;

    fn query(&self) -> Document {
        doc! {
            "rlm": self.realm.to_str(),
            "aid": self.account_id,
            "tid": self.tank_id,
            "lbts": self.last_battle_time,
        }
    }

    fn update(&self) -> Result<Self::Update> {
        Ok(doc! { "$setOnInsert": bson::to_bson(self)? })
    }
}

impl TankSnapshot {
    pub fn from(
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        stats: wargaming::TankStats,
        _achievements: &wargaming::TankAchievements,
    ) -> Self {
        Self {
            realm,
            last_battle_time: stats.last_battle_time,
            account_id,
            tank_id: stats.tank_id,
            battle_life_time: stats.battle_life_time,
            stats: stats.all.into(),
        }
    }

    /// Constructs a vector of tank snapshots from the tanks statistics and achievements.
    pub fn from_vec(
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        mut stats: Vec<wargaming::TankStats>,
        mut achievements: Vec<wargaming::TankAchievements>,
    ) -> Vec<Self> {
        stats.sort_unstable_by_key(|stats| stats.tank_id);
        achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

        merge_join_by(stats, achievements, |left, right| left.tank_id.cmp(&right.tank_id))
            .filter_map(|item| match item {
                EitherOrBoth::Both(stats, achievements) => {
                    Some(Self::from(realm, account_id, stats, &achievements))
                }
                _ => None,
            })
            .collect()
    }

    /// Finds difference between the actual statistics and snapshot's statistics.
    pub fn subtract_collections(
        mut actual_tanks: AHashMap<wargaming::TankId, Self>,
        snapshots: Vec<Self>,
    ) -> Vec<Self> {
        let mut subtracted: Vec<Self> = snapshots
            .into_iter()
            .filter_map(|snapshot| {
                actual_tanks
                    .remove(&snapshot.tank_id)
                    .map(|actual_tank| (snapshot, actual_tank))
            })
            .filter_map(|(snapshot, actual_tank)| {
                (actual_tank.stats.n_battles > snapshot.stats.n_battles)
                    .then(|| actual_tank - snapshot)
            })
            .collect();
        subtracted.extend(
            actual_tanks
                .into_values()
                .filter(|tank| tank.stats.n_battles != 0),
        );
        subtracted
    }
}

impl Sub<TankSnapshot> for TankSnapshot {
    type Output = Self;

    fn sub(self, rhs: TankSnapshot) -> Self::Output {
        Self {
            realm: self.realm,
            last_battle_time: self.last_battle_time,
            account_id: self.account_id,
            tank_id: self.tank_id,
            battle_life_time: self.battle_life_time - rhs.battle_life_time,
            stats: self.stats - rhs.stats,
        }
    }
}

impl TankSnapshot {
    #[instrument(skip_all, level = "debug")]
    pub async fn upsert_many(
        into: &Database,
        snapshots: impl IntoIterator<Item = &Self>,
    ) -> Result {
        let start_instant = Instant::now();
        for snapshot in snapshots {
            snapshot.upsert(into).await?;
        }
        debug!(elapsed = ?start_instant.elapsed());
        Ok(())
    }

    #[instrument(
        skip_all,
        level = "debug",
        fields(account_id = account_id, before = ?before, n_tanks = tank_ids.len()),
    )]
    pub async fn retrieve_latest_tank_snapshots(
        from: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        before: DateTime,
        tank_ids: &[wargaming::TankId],
    ) -> Result<Vec<Self>> {
        if tank_ids.is_empty() {
            return Ok(Vec::new());
        }

        let pipeline = [
            doc! {
                "$match": {
                    "rlm": realm.to_str(),
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
        let collection = Self::collection(from);
        let future = spawn(async move { collection.aggregate(pipeline, None).await });
        let cursor = timeout(time::Duration::from_secs(30), future)
            .await
            .context("timed out to retrieve the latest tanks snapshots")??
            .with_context(|| {
                format!("failed to retrieve the latest tank snapshots for #{}", account_id)
            })?;
        let stream = cursor
            .try_filter_map(|document| async move {
                trace!(?document);
                Ok(Some(from_document::<Root<Self>>(document)?.root))
            })
            .try_collect::<Vec<Self>>()
            .await?;

        debug!(elapsed = format_elapsed(start_instant).as_str(), "done");
        Ok(stream)
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn retrieve_many(
        from: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        tank_last_battle_times: impl IntoIterator<Item = &TankLastBattleTime>,
    ) -> Result<Vec<Self>> {
        let start_instant = Instant::now();
        let or_clauses = tank_last_battle_times
            .into_iter()
            .map(|item| {
                doc! {
                    "tid": item.tank_id,
                    "lbts": item.last_battle_time,
                }
            })
            .collect_vec();
        debug!(n_or_clauses = or_clauses.len());
        trace!(?or_clauses);
        if or_clauses.is_empty() {
            return Ok(Vec::new());
        }
        let snapshots: Vec<Self> = Self::collection(from)
            .find(doc! { "rlm": realm.to_str(), "aid": account_id, "$or": or_clauses }, None)
            .await
            .context("failed to find the snapshots")?
            .try_collect()
            .await?;
        debug!(
            elapsed_secs = start_instant.elapsed().as_secs_f32(),
            n_snapshots = snapshots.len(),
            "done"
        );
        Ok(snapshots)
    }
}
