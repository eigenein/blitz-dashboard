use std::collections::hash_map::Entry;

use either::Either;
use futures::future::try_join;
use itertools::Itertools;
use mongodb::bson;
use poem::error::{InternalServerError, NotFoundError};
use poem::web::{Path, Query};
use poem::Result;

use crate::helpers::sentry::set_user;
use crate::math::statistics::ConfidenceInterval;
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::subtract_tanks;
use crate::web::views::player::models::{Params, Segments};
use crate::{database, wargaming};

pub struct ViewModel {
    pub realm: wargaming::Realm,
    pub actual_info: wargaming::AccountInfo,
    pub current_win_rate: ConfidenceInterval,
    pub battle_life_time: i64,
    pub stats_delta: StatsDelta,
}

impl ViewModel {
    pub async fn new(
        Path(Segments { realm, account_id }): Path<Segments>,
        query: Query<Params>,
        mongodb: &mongodb::Database,
        info_cache: &AccountInfoCache,
        tanks_cache: &AccountTanksCache,
    ) -> Result<Self> {
        let (actual_info, actual_tanks) =
            try_join(info_cache.get(realm, account_id), tanks_cache.get(realm, account_id)).await?;
        let actual_info = actual_info.ok_or(NotFoundError)?;
        set_user(&actual_info.nickname);
        database::Account::new(realm, account_id)
            .upsert(mongodb, database::Account::OPERATION_SET_ON_INSERT)
            .await?;

        let before =
            Utc::now() - Duration::from_std(query.period.0).map_err(InternalServerError)?;
        let current_win_rate = ConfidenceInterval::wilson_score_interval(
            actual_info.statistics.all.n_battles,
            actual_info.statistics.all.n_wins,
            Default::default(),
        );
        let stats_delta = match retrieve_deltas_quickly(
            mongodb,
            realm,
            account_id,
            actual_info.statistics.all,
            actual_info.statistics.rating,
            actual_tanks,
            before,
        )
        .await?
        {
            Either::Left(delta) => delta,
            Either::Right(tanks) => {
                retrieve_deltas_slowly(
                    mongodb,
                    realm,
                    account_id,
                    tanks,
                    before,
                    actual_info.statistics.rating,
                )
                .await?
            }
        };
        let battle_life_time: i64 = stats_delta
            .tanks
            .iter()
            .map(|snapshot| snapshot.battle_life_time.num_seconds())
            .sum();

        Ok(Self {
            realm,
            actual_info,
            current_win_rate,
            battle_life_time,
            stats_delta,
        })
    }
}

pub struct StatsDelta {
    pub random: database::RandomStatsSnapshot,
    pub rating: database::RatingStatsSnapshot,
    pub tanks: Vec<database::TankSnapshot>,
}

#[instrument(skip_all, level = "debug", fields(account_id = account_id, before = ?before))]
async fn retrieve_deltas_quickly(
    from: &mongodb::Database,
    realm: wargaming::Realm,
    account_id: wargaming::AccountId,
    random_stats: wargaming::BasicStatistics,
    rating_stats: wargaming::RatingStatistics,
    mut actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
    before: DateTime,
) -> Result<Either<StatsDelta, AHashMap<wargaming::TankId, wargaming::Tank>>> {
    let account_snapshot =
        match database::AccountSnapshot::retrieve_latest(from, realm, account_id, before).await? {
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
                        let keep = bson::DateTime::from(entry.get().statistics.last_battle_time)
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
    Ok(Either::Left(StatsDelta {
        random: random_stats - account_snapshot.random_stats,
        rating: rating_stats - account_snapshot.rating_stats,
        tanks: subtract_tanks(realm, actual_tanks, snapshots),
    }))
}

#[instrument(skip_all, level = "debug", fields(account_id = account_id))]
async fn retrieve_deltas_slowly(
    from: &mongodb::Database,
    realm: wargaming::Realm,
    account_id: wargaming::AccountId,
    actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
    before: DateTime,
    rating_stats: wargaming::RatingStatistics,
) -> Result<StatsDelta> {
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
    Ok(StatsDelta {
        random: tanks_delta.iter().map(|tank| tank.stats).sum(),
        rating: rating_stats.into(),
        tanks: tanks_delta,
    })
}
