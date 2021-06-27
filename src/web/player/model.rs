use std::ops::Sub;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use async_std::sync::Mutex;
use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use lru_time_cache::LruCache;
use serde::Deserialize;
use tide::Request;

use crate::logging::log_anyhow;
use crate::models::{AccountInfo, AllStatistics, TankSnapshot};
use crate::statistics::ConfidenceInterval;
use crate::web::state::State;
use itertools::{merge_join_by, EitherOrBoth};

/// Defines the model cache lookup key. Consists of account ID and display period.
type ModelCacheKey = (i32, Period);

lazy_static! {
    static ref MODEL_CACHE: Arc<Mutex<LruCache<ModelCacheKey, Arc<PlayerViewModel>>>> =
        Arc::new(Mutex::new(LruCache::with_expiry_duration_and_capacity(
            std::time::Duration::from_secs(90),
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
    pub period: Period,
    pub wins: Option<ConfidenceInterval>,
    pub survival: Option<ConfidenceInterval>,
    pub hits: Option<ConfidenceInterval>,
    pub damage_dealt_mean: f64,
    pub warn_no_previous_account_info: bool,
    pub statistics: AllStatistics,
    pub tank_snapshots: Vec<TankSnapshot>,
}

impl PlayerViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<Arc<PlayerViewModel>> {
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

        let mut cache = MODEL_CACHE.lock().await;
        let model = match cache.get(&(account_id, query.period)) {
            Some(model) => model.clone(),
            None => {
                let model =
                    Arc::new(Self::new_uncached(request.state(), account_id, query.period).await?);
                cache.insert((account_id, query.period), model.clone());
                model
            }
        };
        Ok(model)
    }

    /// Constructs a model from scratch.
    async fn new_uncached(state: &State, account_id: i32, period: Period) -> crate::Result<Self> {
        let account_info = Arc::new(
            state
                .api
                .get_account_info(account_id)
                .await?
                .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
        );
        if account_info.is_active() {
            Self::upsert_account(&state, &account_info);
        }

        let actual_statistics = &account_info.statistics.all;
        let actual_tanks = state.api.get_merged_tanks(account_id).await?;
        let total_tanks = actual_tanks.len();
        let before = DateTime::<Utc>::from(&period);
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
        let mut tank_snapshots = Self::subtract_tank_snapshots(actual_tanks, previous_tanks);
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
            period,
            wins: ConfidenceInterval::from_proportion_90(statistics.battles, statistics.wins),
            survival: ConfidenceInterval::from_proportion_90(
                statistics.battles,
                statistics.survived_battles,
            ),
            hits: ConfidenceInterval::from_proportion_90(statistics.shots, statistics.hits),
            damage_dealt_mean: statistics.damage_dealt as f64 / statistics.battles.max(1) as f64,
            warn_no_previous_account_info,
            statistics,
            tank_snapshots,
            total_tanks,
        })
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

    /// Upserts account info in background so that it could be picked up by the crawler.
    fn upsert_account(state: &State, account_info: &Arc<AccountInfo>) {
        let account_info = account_info.clone();
        let database = state.database.clone();
        async_std::task::spawn(async move {
            log_anyhow(database.lock().await.upsert_account(&account_info.basic));
        });
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

#[derive(Deserialize)]
struct Query {
    #[serde(default)]
    period: Period,
}

#[derive(Deserialize, PartialEq, Clone, Ord, PartialOrd, Eq, Copy, Debug)]
pub enum Period {
    #[serde(rename = "1h")]
    Hour,

    #[serde(rename = "4h")]
    FourHours,

    #[serde(rename = "8h")]
    EightHours,

    #[serde(rename = "12h")]
    TwelveHours,

    #[serde(rename = "1d")]
    Day,

    #[serde(rename = "1w")]
    Week,

    #[serde(rename = "1m")]
    Month,

    #[serde(rename = "1y")]
    Year,
}

impl Default for Period {
    fn default() -> Self {
        Self::TwelveHours
    }
}

impl From<&Period> for Duration {
    fn from(since: &Period) -> Self {
        match since {
            Period::Hour => Self::hours(1),
            Period::FourHours => Self::hours(4),
            Period::EightHours => Self::hours(8),
            Period::TwelveHours => Self::hours(12),
            Period::Day => Self::days(1),
            Period::Week => Self::weeks(1),
            Period::Month => Self::days(30),
            Period::Year => Self::days(365),
        }
    }
}

impl From<&Period> for DateTime<Utc> {
    fn from(since: &Period) -> Self {
        Utc::now() - Duration::from(since)
    }
}
