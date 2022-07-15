use std::collections::BTreeMap;
use std::net::IpAddr;

use futures::future::try_join;
use itertools::Itertools;
use poem::error::{InternalServerError, NotFoundError};
use poem::web::{Path, Query};
use sentry::protocol::IpAddress;

use crate::math::statistics::ConfidenceInterval;
use crate::math::traits::{MMRating, TrueWinRate};
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::web::views::player::models::{Params, Segments, StatsDelta};
use crate::{database, wargaming};

pub struct ViewModel {
    pub realm: wargaming::Realm,
    pub actual_info: wargaming::AccountInfo,
    pub current_win_rate: ConfidenceInterval,
    pub battle_life_time_secs: f64,
    pub stats_delta: StatsDelta,
    pub rating_snapshots_data: Vec<(i64, i32)>,
}

impl ViewModel {
    pub async fn new(
        ip_addr: Option<IpAddr>,
        Path(Segments { realm, account_id }): Path<Segments>,
        query: Query<Params>,
        mongodb: &mongodb::Database,
        info_cache: &AccountInfoCache,
        tanks_cache: &AccountTanksCache,
    ) -> poem::Result<Self> {
        let mut user =
            Self::get_sentry_user(realm, account_id, ip_addr).map_err(poem::Error::from)?;
        sentry::configure_scope(|scope| scope.set_user(Some(user.clone())));

        let (actual_info, actual_tanks) =
            try_join(info_cache.get(realm, account_id), tanks_cache.get(realm, account_id)).await?;
        let actual_info = actual_info.ok_or(NotFoundError)?;

        let last_known_battle_time = database::Account::new(realm, account_id)
            .ensure_exists(mongodb)
            .await?
            .last_battle_time;
        // Do not crawl unknown accounts.
        if let Some(last_known_battle_time) = last_known_battle_time {
            // Only crawl when the last battle time has been updated.
            if actual_info.last_battle_time > last_known_battle_time {
                Self::crawl_account(
                    mongodb,
                    realm,
                    &actual_info,
                    last_known_battle_time,
                    &actual_tanks,
                )
                .await
                .map_err(poem::Error::from)?;
            }
        }

        // Now that we know the user's nickname, update the Sentry user.
        user.username = Some(actual_info.nickname.clone());
        sentry::configure_scope(|scope| scope.set_user(Some(user)));

        let current_win_rate = actual_info.stats.random.true_win_rate();

        let before =
            Utc::now() - Duration::from_std(query.period.0).map_err(InternalServerError)?;
        let stats_delta = StatsDelta::retrieve(
            mongodb,
            realm,
            account_id,
            actual_info.stats,
            actual_tanks,
            before,
        )
        .await?;

        let battle_life_time_secs = stats_delta
            .tanks
            .iter()
            .map(|snapshot| snapshot.battle_life_time.num_seconds())
            .sum::<i64>() as f64;

        let rating_snapshots_data = Self::retrieve_rating_snapshots_data(
            mongodb,
            realm,
            account_id,
            actual_info.last_battle_time,
            actual_info.stats.rating.mm_rating,
        )
        .await
        .map_err(poem::Error::from)?;

        Ok(Self {
            realm,
            actual_info,
            current_win_rate,
            battle_life_time_secs,
            stats_delta,
            rating_snapshots_data,
        })
    }

    /// Retrieves the most recent rating snapshots from the account snapshots.
    /// This is used to display the rating chart.
    async fn retrieve_rating_snapshots_data(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        last_battle_time: DateTime,
        actual_mm_rating: f64,
    ) -> Result<Vec<(i64, i32)>> {
        let mut rating_snapshots =
            database::RatingSnapshot::retrieve_latest(from, realm, account_id, 10).await?;
        rating_snapshots.reverse();
        if rating_snapshots
            .last()
            .map(|snapshot| snapshot.last_battle_time < last_battle_time)
            .unwrap_or_default()
        {
            rating_snapshots.push(database::RatingSnapshot {
                last_battle_time,
                mm_rating: actual_mm_rating,
            });
        }
        let rating_snapshots_data = rating_snapshots
            .into_iter()
            .map(|snapshot| {
                (snapshot.last_battle_time.timestamp_millis(), snapshot.display_rating())
            })
            .collect();
        Ok(rating_snapshots_data)
    }

    /// Crawls the account while browsing the website.
    /// This is similar to [`crate::crawler::update_account`].
    async fn crawl_account(
        into: &mongodb::Database,
        realm: wargaming::Realm,
        account_info: &wargaming::AccountInfo,
        last_known_battle_time: DateTime,
        tank_snapshots: &AHashMap<wargaming::TankId, database::TankSnapshot>,
    ) -> Result {
        for tank_snapshot in tank_snapshots.values() {
            if tank_snapshot.last_battle_time > last_known_battle_time {
                tank_snapshot.upsert(into).await?;
            }
        }
        let tank_last_battle_times = tank_snapshots
            .values()
            .map_into::<database::TankLastBattleTime>()
            .collect_vec();
        database::AccountSnapshot::new(realm, account_info, tank_last_battle_times)
            .upsert(into)
            .await?;
        database::Account::new(realm, account_info.id)
            .last_battle_time(account_info.last_battle_time)
            .upsert(into)
            .await?;
        Ok(())
    }

    /// Instantiates a Sentry user from the account.
    fn get_sentry_user(
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        ip_addr: Option<IpAddr>,
    ) -> Result<sentry::User> {
        Ok(sentry::User {
            id: Some(account_id.to_string()),
            ip_address: ip_addr.map(IpAddress::Exact),
            other: BTreeMap::from([("realm".to_string(), serde_json::to_value(realm)?)]),
            ..Default::default()
        })
    }
}
