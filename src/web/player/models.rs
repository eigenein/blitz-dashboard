use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context};
use async_std::sync::Mutex;
use chrono::{DateTime, Duration, Utc};
use itertools::{merge_join_by, EitherOrBoth};
use lazy_static::lazy_static;
use lru_time_cache::LruCache;
use serde::{Deserialize, Serialize};
use tide::Request;

use crate::models::{AccountInfo, AllStatistics, TankSnapshot};
use crate::wargaming::WargamingApi;
use crate::web::state::State;

lazy_static! {
    static ref ACCOUNT_INFO_CACHE: Arc<Mutex<LruCache<i32, Arc<AccountInfo>>>> =
        Arc::new(Mutex::new(LruCache::with_expiry_duration_and_capacity(
            std::time::Duration::from_secs(60),
            1000,
        )));
    static ref ACCOUNT_TANKS_CACHE: Arc<Mutex<LruCache<i32, Arc<Vec<TankSnapshot>>>>> =
        Arc::new(Mutex::new(LruCache::with_expiry_duration_and_capacity(
            std::time::Duration::from_secs(60),
            1000,
        )));
}

pub struct PlayerViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub has_recently_played: bool,
    pub is_active: bool,
    pub total_battles: i32,
    pub total_tanks: usize,
    pub query: Query,
    pub warn_no_previous_account_info: bool,
    pub statistics: AllStatistics,
    pub tank_snapshots: Vec<TankSnapshot>,
}

impl PlayerViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<PlayerViewModel> {
        let account_id = Self::parse_account_id(&request)?;
        let query = request
            .query::<Query>()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?;
        log::info!(
            "Requested player #{} within {:?}.",
            account_id,
            query.period,
        );

        let state = request.state();
        let account_info = Self::get_cached_account_info(&state.api, account_id).await?;
        if account_info.is_active() {
            Self::insert_account_or_ignore(&state, &account_info).await?;
        }

        let actual_statistics = &account_info.statistics.all;
        let actual_tanks = Self::get_cached_tank_snapshots(&state.api, account_id).await?;
        let total_tanks = actual_tanks.len();
        let before = Utc::now() - Duration::from_std(query.period)?;
        let previous_account_info = {
            let database = state.database.clone();
            async_std::task::spawn(async move {
                let database = database.lock().await;
                database.retrieve_latest_account_snapshot(account_id, &before)
            })
            .await?
        };
        let previous_tanks = if previous_account_info.is_some() {
            let database = state.database.clone();
            async_std::task::spawn(async move {
                let database = database.lock().await;
                database.retrieve_latest_tank_snapshots(account_id, &before)
            })
            .await?
        } else {
            Vec::new()
        };
        let mut tank_snapshots =
            Self::subtract_tank_snapshots(actual_tanks.to_vec(), previous_tanks);
        tank_snapshots.sort_by_key(|snapshot| -snapshot.all_statistics.battles);
        let warn_no_previous_account_info = previous_account_info.is_none();
        let statistics = actual_statistics
            .sub(&previous_account_info.map_or_else(Default::default, |info| info.statistics.all));

        Ok(Self {
            account_id: account_info.basic.id,
            nickname: account_info.nickname.clone(),
            created_at: account_info.created_at,
            last_battle_time: account_info.basic.last_battle_time,
            total_battles: account_info.statistics.all.battles,
            has_recently_played: account_info.basic.last_battle_time
                > (Utc::now() - Duration::hours(1)),
            is_active: account_info.is_active(),
            query,
            warn_no_previous_account_info,
            statistics,
            tank_snapshots,
            total_tanks,
        })
    }

    async fn get_cached_account_info(
        api: &WargamingApi,
        account_id: i32,
    ) -> crate::Result<Arc<AccountInfo>> {
        let mut cache = ACCOUNT_INFO_CACHE.lock().await;
        match cache.get(&account_id) {
            Some(account_info) => {
                log::debug!("Cache hit on account #{} info.", account_id);
                Ok(account_info.clone())
            }
            None => {
                let account_info = Arc::new(
                    api.get_account_info([account_id])
                        .await?
                        .remove(&account_id.to_string())
                        .flatten()
                        .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
                );
                cache.insert(account_id, account_info.clone());
                Ok(account_info)
            }
        }
    }

    async fn get_cached_tank_snapshots(
        api: &WargamingApi,
        account_id: i32,
    ) -> crate::Result<Arc<Vec<TankSnapshot>>> {
        let mut cache = ACCOUNT_TANKS_CACHE.lock().await;
        match cache.get(&account_id) {
            Some(snapshots) => {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots.clone())
            }
            None => {
                let snapshots = Arc::new(api.get_merged_tanks(account_id).await?);
                cache.insert(account_id, snapshots.clone());
                Ok(snapshots)
            }
        }
    }

    fn subtract_tank_snapshots(
        mut actual_snapshots: Vec<TankSnapshot>,
        mut previous_snapshots: Vec<TankSnapshot>,
    ) -> Vec<TankSnapshot> {
        actual_snapshots.sort_by_key(|snapshot| snapshot.tank_id);
        previous_snapshots.sort_by_key(|snapshot| snapshot.tank_id);

        merge_join_by(actual_snapshots, previous_snapshots, |left, right| {
            left.tank_id.cmp(&right.tank_id)
        })
        .filter_map(|item| match item {
            EitherOrBoth::Both(actual, previous)
                if actual.all_statistics.battles > previous.all_statistics.battles =>
            {
                Some(TankSnapshot {
                    account_id: actual.account_id,
                    tank_id: actual.tank_id,
                    achievements: Default::default(), // TODO
                    max_series: Default::default(),   // TODO
                    all_statistics: actual.all_statistics.sub(&previous.all_statistics),
                    last_battle_time: actual.last_battle_time,
                    battle_life_time: actual.battle_life_time - previous.battle_life_time,
                })
            }
            EitherOrBoth::Left(actual) => Some(actual),
            _ => None,
        })
        .collect::<Vec<TankSnapshot>>()
    }

    /// Inserts account if it doesn't exist. The rest is updated by [`crate::crawler`].
    async fn insert_account_or_ignore(
        state: &State,
        account_info: &Arc<AccountInfo>,
    ) -> crate::Result {
        let account_info = account_info.clone();
        let database = state.database.clone();
        async_std::task::spawn(async move {
            database
                .lock()
                .await
                .insert_account_or_ignore(&account_info.basic)
        })
        .await?;
        Ok(())
    }

    /// Parses account ID URL segment.
    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        request
            .param("account_id")
            .map_err(surf::Error::into_inner)
            .context("missing account ID")?
            .parse()
            .context("invalid account ID")
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Query {
    #[serde(default = "default_period", with = "humantime_serde")]
    pub period: StdDuration,

    #[serde(default = "default_sort_by", rename = "sort-by")]
    pub sort_by: SortBy,
}

fn default_period() -> StdDuration {
    StdDuration::from_secs(86400)
}

fn default_sort_by() -> SortBy {
    SortBy::Battles
}

impl Query {
    pub fn with_period(&self, period: StdDuration) -> Self {
        Self {
            period,
            sort_by: self.sort_by,
        }
    }

    pub fn with_sort_by(&self, sort_by: SortBy) -> Self {
        Self {
            sort_by,
            period: self.period,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum SortBy {
    #[serde(rename = "battles")]
    Battles,
}
