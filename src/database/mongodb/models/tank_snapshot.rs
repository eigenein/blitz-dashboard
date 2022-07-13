use std::ops::Sub;

use futures::TryStreamExt;
use itertools::{merge_join_by, EitherOrBoth, Itertools};
use mongodb::bson::{doc, from_document};
use mongodb::options::{IndexOptions, UpdateOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

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

    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde(rename = "tid")]
    pub tank_id: wargaming::TankId,

    #[serde(rename = "life")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub battle_life_time: Duration,

    #[serde(flatten)]
    pub stats: RandomStatsSnapshot,
}

impl TankSnapshot {
    pub fn from(
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        stats: wargaming::TankStats,
        _achievements: wargaming::TankAchievements,
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

    pub fn from_vec(
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        mut stats: Vec<wargaming::TankStats>,
        mut achievements: Vec<wargaming::TankAchievements>,
    ) -> AHashMap<wargaming::TankId, Self> {
        stats.sort_unstable_by_key(|stats| stats.tank_id);
        achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

        merge_join_by(stats, achievements, |left, right| left.tank_id.cmp(&right.tank_id))
            .filter_map(|item| match item {
                EitherOrBoth::Both(stats, achievements) => {
                    Some((stats.tank_id, Self::from(realm, account_id, stats, achievements)))
                }
                _ => None,
            })
            .collect()
    }

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
                (actual_tank.stats.n_battles != snapshot.stats.n_battles)
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
    #[instrument(skip_all)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        let indexes = [
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "aid": 1, "tid": 1, "lbts": -1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "rlm": 1, "aid": 1, "lbts": -1 })
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
            "rlm": self.realm.to_str(),
            "aid": self.account_id,
            "tid": self.tank_id,
            "lbts": self.last_battle_time,
        };
        let update = doc! { "$setOnInsert": bson::to_bson(self)? };
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
                Ok(Some(document.root))
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
            .map(|item| doc! { "tid": item.tank_id, "lbts": item.last_battle_time})
            .collect_vec();
        debug!(n_or_clauses = or_clauses.len());
        if or_clauses.is_empty() {
            return Ok(Vec::new());
        }
        let snapshots: Vec<Self> = Self::collection(from)
            .find(doc! { "rlm": realm.to_str(), "aid": account_id, "$or": or_clauses }, None)
            .await?
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

impl TankSnapshot {
    #[must_use]
    #[inline]
    pub fn wins_per_hour(&self) -> f64 {
        self.stats.n_wins as f64 / self.battle_life_time.num_seconds() as f64 * 3600.0
    }

    #[must_use]
    #[inline]
    pub fn battles_per_hour(&self) -> f64 {
        self.stats.n_battles as f64 / self.battle_life_time.num_seconds() as f64 * 3600.0
    }

    #[must_use]
    #[inline]
    pub fn damage_per_minute(&self) -> f64 {
        self.stats.damage_dealt as f64 / self.battle_life_time.num_seconds() as f64 * 60.0
    }
}

impl TankSnapshot {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("tank_snapshots")
    }
}
