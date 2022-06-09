use mongodb::bson::doc;
use mongodb::options::{IndexOptions, UpdateModifications, UpdateOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::helpers::tracing::format_elapsed;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
pub struct TankSnapshot {
    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,

    #[serde(rename = "aid")]
    pub account_id: i32,

    #[serde(rename = "tid")]
    pub tank_id: u32,

    #[serde(rename = "life")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub battle_life_time: Duration,

    #[serde(rename = "nb")]
    pub n_battles: u32,

    #[serde(rename = "nw")]
    pub n_wins: u32,

    #[serde(rename = "nsb")]
    pub n_survived_battles: u32,

    #[serde(rename = "nws")]
    pub n_win_and_survived: u32,

    #[serde(rename = "dmgd")]
    pub damage_dealt: u32,

    #[serde(rename = "dmgr")]
    pub damage_received: u32,

    #[serde(rename = "shts")]
    pub n_shots: u32,

    #[serde(rename = "hits")]
    pub n_hits: u32,

    #[serde(rename = "frgs")]
    pub n_frags: u32,

    #[serde(rename = "xp")]
    pub xp: u32,
}

impl From<wargaming::Tank> for TankSnapshot {
    fn from(tank: wargaming::Tank) -> Self {
        Self {
            last_battle_time: tank.statistics.basic.last_battle_time,
            account_id: tank.account_id,
            tank_id: tank.statistics.basic.tank_id as u32,
            battle_life_time: tank.statistics.battle_life_time,
            n_battles: tank.statistics.all.battles as u32,
            n_wins: tank.statistics.all.wins as u32,
            n_survived_battles: tank.statistics.all.survived_battles as u32,
            n_win_and_survived: tank.statistics.all.win_and_survived as u32,
            damage_dealt: tank.statistics.all.damage_dealt as u32,
            damage_received: tank.statistics.all.damage_received as u32,
            n_shots: tank.statistics.all.shots as u32,
            n_hits: tank.statistics.all.hits as u32,
            n_frags: tank.statistics.all.frags as u32,
            xp: tank.statistics.all.xp as u32,
        }
    }
}

impl TankSnapshot {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("tank_snapshots")
    }

    #[instrument(skip_all)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        let indexes = [IndexModel::builder()
            .keys(doc! { "lbts": -1, "aid": 1, "tid": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build()];
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
}
