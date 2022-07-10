use std::collections::hash_map::Entry;

use either::Either;
use itertools::Itertools;
use mongodb::bson;

use crate::prelude::*;
use crate::wargaming::subtract_tanks;
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
        random_stats: wargaming::BasicStats,
        rating_stats: wargaming::RatingStats,
        actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
        before: DateTime,
    ) -> Result<Self> {
        let this = match Self::retrieve_quickly(
            from,
            realm,
            account_id,
            random_stats,
            rating_stats,
            actual_tanks,
            before,
        )
        .await?
        {
            Either::Left(delta) => delta,
            Either::Right(tanks) => {
                Self::retrieve_slowly(from, realm, account_id, tanks, before, rating_stats).await?
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
        random_stats: wargaming::BasicStats,
        rating_stats: wargaming::RatingStats,
        mut actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
        before: DateTime,
    ) -> Result<Either<Self, AHashMap<wargaming::TankId, wargaming::Tank>>> {
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
                .filter(|(tank_id, last_battle_time)| {
                    let tank_entry = actual_tanks.entry(*tank_id);
                    match tank_entry {
                        Entry::Occupied(entry) => {
                            let keep =
                                bson::DateTime::from(entry.get().statistics.last_battle_time)
                                    > *last_battle_time;
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
            random: random_stats - account_snapshot.random_stats,
            rating: rating_stats - account_snapshot.rating_stats,
            tanks: subtract_tanks(realm, actual_tanks, snapshots),
        }))
    }

    #[instrument(skip_all, level = "debug", fields(realm = ?realm, account_id = account_id))]
    async fn retrieve_slowly(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
        before: DateTime,
        rating_stats: wargaming::RatingStats,
    ) -> Result<Self> {
        debug!("taking the slow path");
        let actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank> = actual_tanks
            .into_iter()
            .filter(|(_, tank)| tank.statistics.last_battle_time >= before)
            .collect();
        let snapshots = {
            let tank_ids = actual_tanks
                .values()
                .map(wargaming::Tank::tank_id)
                .collect_vec();
            database::TankSnapshot::retrieve_latest_tank_snapshots(
                from, realm, account_id, before, &tank_ids,
            )
            .await?
        };
        let tanks_delta = subtract_tanks(realm, actual_tanks, snapshots);
        Ok(Self {
            random: tanks_delta.iter().map(|tank| tank.stats).sum(),
            rating: rating_stats.into(),
            tanks: tanks_delta,
        })
    }
}
