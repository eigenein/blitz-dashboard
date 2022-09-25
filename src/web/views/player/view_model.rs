use std::collections::BTreeMap;
use std::net::IpAddr;

use bpci::BoundedInterval;
use futures::future::try_join;
use poem::error::{InternalServerError, NotFoundError};
use poem::web::cookie::CookieJar;
use poem::web::Path;
use sentry::protocol::IpAddress;

use crate::math::traits::*;
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::web::views::player::display_preferences::DisplayPreferences;
use crate::web::views::player::path::PathSegments;
use crate::web::views::player::stats_delta::StatsDelta;
use crate::{database, wargaming};

pub struct ViewModel {
    pub realm: wargaming::Realm,
    pub actual_info: wargaming::AccountInfo,
    pub current_win_rate: BoundedInterval<f64>,
    pub target_victory_ratio: f64,
    pub stats_delta: StatsDelta,
    pub rating_snapshots: Vec<database::RatingSnapshot>,
    pub preferences: DisplayPreferences,
}

impl ViewModel {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        ip_addr: Option<IpAddr>,
        Path(PathSegments { realm, account_id }): Path<PathSegments>,
        cookies: &CookieJar,
        db: &mongodb::Database,
        info_cache: &AccountInfoCache,
        tanks_cache: &AccountTanksCache,
    ) -> poem::Result<Self> {
        let mut user =
            Self::get_sentry_user(realm, account_id, ip_addr).map_err(poem::Error::from)?;
        sentry::configure_scope(|scope| scope.set_user(Some(user.clone())));

        let (actual_info, actual_tanks) =
            try_join(info_cache.get(realm, account_id), tanks_cache.get(realm, account_id)).await?;
        let actual_info = actual_info.ok_or(NotFoundError)?;

        database::Account::ensure_exists(db, realm, account_id)
            .await
            .context("failed to ensure the account existence")?;

        // Now that we know the user's nickname, update the Sentry user.
        user.username = Some(actual_info.nickname.clone());
        sentry::configure_scope(|scope| scope.set_user(Some(user)));

        let preferences = DisplayPreferences::from(cookies);
        let current_win_rate = actual_info
            .stats
            .random
            .victory_ratio_interval(preferences.confidence_z_level)?;
        let target_victory_ratio = preferences
            .target_victory_ratio
            .custom_or_else(|| actual_info.stats.random.victory_ratio());
        let before =
            Utc::now() - Duration::from_std(preferences.period).map_err(InternalServerError)?;
        let stats_delta =
            StatsDelta::retrieve(db, realm, account_id, &actual_info.stats, actual_tanks, before)
                .await?;

        let rating_snapshots = database::RatingSnapshot::retrieve_season(
            db,
            realm,
            account_id,
            actual_info.stats.rating.current_season,
        )
        .await?;

        Ok(Self {
            realm,
            actual_info,
            current_win_rate,
            stats_delta,
            rating_snapshots,
            preferences,
            target_victory_ratio,
        })
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
