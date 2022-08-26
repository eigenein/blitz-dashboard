use std::collections::hash_map::Entry;

use either::Either;
use itertools::Itertools;

use crate::prelude::*;
use crate::{database, wargaming};

pub struct StatsDelta {
    pub random: database::RandomStatsSnapshot,
    pub rating: database::RatingStatsSnapshot,
    pub tanks: Vec<database::TankSnapshot>,
}

impl StatsDelta {
    #[instrument(
        skip_all,
        level = "debug",
        fields(realm = ?realm, account_id = account_id, before = ?before),
    )]
    pub async fn retrieve(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        stats: &wargaming::AccountInfoStats,
        actual_tanks: AHashMap<wargaming::TankId, database::TankSnapshot>,
        before: DateTime,
    ) -> Result<Self> {
        let this =
            match Self::retrieve_quickly(from, realm, account_id, stats, actual_tanks, before)
                .await?
            {
                Either::Left(delta) => delta,
                Either::Right(tanks) => {
                    Self::retrieve_slowly(from, realm, account_id, tanks, before, &stats.rating)
                        .await?
                }
            };
        Ok(this)
    }

    #[instrument(
        skip_all,
        level = "debug",
        fields(realm = ?realm, account_id = account_id, before = ?before),
    )]
    async fn retrieve_quickly(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        stats: &wargaming::AccountInfoStats,
        mut actual_tanks: AHashMap<wargaming::TankId, database::TankSnapshot>,
        before: DateTime,
    ) -> Result<Either<Self, AHashMap<wargaming::TankId, database::TankSnapshot>>> {
        let account_snapshot =
            match database::AccountSnapshot::retrieve_latest(from, realm, account_id, before)
                .await?
            {
                Some(account_snapshot) => account_snapshot,
                None => return Ok(Either::Right(actual_tanks)),
            };
        let tank_last_battle_times =
            account_snapshot
                .tank_last_battle_times
                .iter()
                .filter(|item| {
                    let tank_entry = actual_tanks.entry(item.tank_id);
                    match tank_entry {
                        Entry::Occupied(entry) => {
                            let keep = entry.get().last_battle_time > item.last_battle_time;
                            if !keep {
                                entry.remove();
                            }
                            keep
                        }
                        Entry::Vacant(_) => false,
                    }
                });
        let snapshots =
            database::TankSnapshot::retrieve_many(from, realm, account_id, tank_last_battle_times)
                .await?;
        Ok(Either::Left(Self {
            random: stats.random - account_snapshot.random_stats,
            rating: stats.rating - account_snapshot.rating_stats,
            tanks: database::TankSnapshot::subtract_collections(actual_tanks, snapshots),
        }))
    }

    #[instrument(skip_all, level = "debug", fields(realm = ?realm, account_id = account_id))]
    async fn retrieve_slowly(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        actual_tanks: AHashMap<wargaming::TankId, database::TankSnapshot>,
        before: DateTime,
        rating_stats: &wargaming::RatingStats,
    ) -> Result<Self> {
        debug!("taking the slow path");
        let actual_tanks: AHashMap<_, _> = actual_tanks
            .into_iter()
            .filter(|(_, tank)| tank.last_battle_time >= before)
            .collect();
        let snapshots = {
            let tank_ids = actual_tanks
                .values()
                .map(|snapshot| snapshot.tank_id)
                .collect_vec();
            database::TankSnapshot::retrieve_latest_tank_snapshots(
                from, realm, account_id, before, &tank_ids,
            )
            .await?
        };
        let tanks_delta = database::TankSnapshot::subtract_collections(actual_tanks, snapshots);
        Ok(Self {
            random: tanks_delta.iter().map(|tank| tank.stats).sum(),
            rating: rating_stats.into(),
            tanks: tanks_delta,
        })
    }
}
