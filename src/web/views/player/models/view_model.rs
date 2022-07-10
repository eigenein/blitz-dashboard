use std::collections::BTreeMap;
use std::net::IpAddr;

use futures::future::try_join;
use poem::error::{InternalServerError, NotFoundError};
use poem::web::{Path, Query};
use poem::Result;
use sentry::protocol::IpAddress;

use crate::math::statistics::ConfidenceInterval;
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
}

impl ViewModel {
    pub async fn new(
        ip_addr: Option<IpAddr>,
        Path(Segments { realm, account_id }): Path<Segments>,
        query: Query<Params>,
        mongodb: &mongodb::Database,
        info_cache: &AccountInfoCache,
        tanks_cache: &AccountTanksCache,
    ) -> Result<Self> {
        let mut user = get_sentry_user(realm, account_id, ip_addr)?;
        sentry::configure_scope(|scope| scope.set_user(Some(user.clone())));

        let (actual_info, actual_tanks) =
            try_join(info_cache.get(realm, account_id), tanks_cache.get(realm, account_id)).await?;
        let actual_info = actual_info.ok_or(NotFoundError)?;

        user.username = Some(actual_info.nickname.clone());
        sentry::configure_scope(|scope| scope.set_user(Some(user)));

        database::Account::new(realm, account_id)
            .upsert(mongodb, database::Account::OPERATION_SET_ON_INSERT)
            .await?;

        let before =
            Utc::now() - Duration::from_std(query.period.0).map_err(InternalServerError)?;
        let current_win_rate = ConfidenceInterval::wilson_score_interval(
            actual_info.stats.random.n_battles,
            actual_info.stats.random.n_wins,
            Default::default(),
        );
        let stats_delta = StatsDelta::retrieve(
            mongodb,
            realm,
            account_id,
            actual_info.stats.random,
            actual_info.stats.rating,
            actual_tanks,
            before,
        )
        .await?;
        let battle_life_time_secs = stats_delta
            .tanks
            .iter()
            .map(|snapshot| snapshot.battle_life_time.num_seconds())
            .sum::<i64>() as f64;

        Ok(Self {
            realm,
            actual_info,
            current_win_rate,
            battle_life_time_secs,
            stats_delta,
        })
    }
}

fn get_sentry_user(
    realm: wargaming::Realm,
    account_id: wargaming::AccountId,
    ip_addr: Option<IpAddr>,
) -> Result<sentry::User> {
    Ok(sentry::User {
        id: Some(account_id.to_string()),
        ip_address: ip_addr.map(IpAddress::Exact),
        other: BTreeMap::from([(
            "realm".to_string(),
            serde_json::to_value(realm).map_err(InternalServerError)?,
        )]),
        ..Default::default()
    })
}
