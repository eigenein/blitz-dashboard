use std::cmp::Ordering;
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, Utc};
use itertools::{merge_join_by, EitherOrBoth};
use lazy_static::lazy_static;
use moka::future::{Cache, CacheBuilder};
use serde::{Deserialize, Serialize};
use tide::Request;

use crate::database::async_db;
use crate::models::{AccountInfo, AllStatistics, TankSnapshot, Vehicle};
use crate::statistics::wilson_score_interval_90;
use crate::wargaming::WargamingApi;
use crate::web::state::State;

lazy_static! {
    static ref ACCOUNT_INFO_CACHE: Cache<i32, Arc<AccountInfo>> = CacheBuilder::new(1_000)
        .time_to_live(StdDuration::from_secs(60))
        .build();
    static ref ACCOUNT_TANKS_CACHE: Cache<i32, Arc<Vec<TankSnapshot>>> = CacheBuilder::new(1_000)
        .time_to_live(StdDuration::from_secs(60))
        .build();
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
    pub rows: Vec<DisplayRow>,
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
            let account_info = account_info.clone();
            async_db(&state.database, move |database| {
                database.insert_account_or_ignore(&account_info.basic)
            })
            .await?;
        }

        let actual_statistics = &account_info.statistics.all;
        let actual_tanks = Self::get_cached_tank_snapshots(&state.api, account_id).await?;
        let total_tanks = actual_tanks.len();
        let before = Utc::now() - Duration::from_std(query.period)?;
        let previous_account_info = async_db(&state.database, move |database| {
            database.retrieve_latest_account_snapshot(account_id, &before)
        })
        .await?;
        let previous_tanks = if previous_account_info.is_some() {
            async_db(&state.database, move |database| {
                database.retrieve_latest_tank_snapshots(account_id, &before)
            })
            .await?
        } else {
            Vec::new()
        };
        let tank_snapshots = Self::subtract_tank_snapshots(actual_tanks.to_vec(), previous_tanks);

        let mut rows: Vec<DisplayRow> = Vec::new();
        for snapshot in tank_snapshots.into_iter() {
            rows.push(Self::make_display_row(
                state.get_vehicle(snapshot.tank_id).await?.clone(),
                snapshot,
            )?);
        }
        Self::sort_vehicles(&mut rows, query.sort_by);

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
            rows,
            total_tanks,
        })
    }

    fn make_display_row(
        vehicle: Arc<Vehicle>,
        snapshot: TankSnapshot,
    ) -> crate::Result<DisplayRow> {
        let stats = &snapshot.all_statistics;
        let win_rate = stats.wins as f64 / stats.battles as f64;
        let true_win_rate = wilson_score_interval_90(stats.battles, stats.wins);
        Ok(DisplayRow {
            win_rate,
            true_win_rate,
            damage_per_battle: stats.damage_dealt as f64 / stats.battles as f64,
            survival_rate: stats.survived_battles as f64 / stats.battles as f64,
            all_statistics: snapshot.all_statistics,
            gold_per_battle: 10.0 + vehicle.tier as f64 * win_rate,
            true_gold_per_battle: 10.0 + vehicle.tier as f64 * true_win_rate.0,
            vehicle,
        })
    }

    async fn get_cached_account_info(
        api: &WargamingApi,
        account_id: i32,
    ) -> crate::Result<Arc<AccountInfo>> {
        match ACCOUNT_INFO_CACHE.get(&account_id) {
            Some(account_info) => {
                log::debug!("Cache hit on account #{} info.", account_id);
                Ok(account_info)
            }
            None => {
                let account_info = Arc::new(
                    api.get_account_info([account_id])
                        .await?
                        .remove(&account_id.to_string())
                        .flatten()
                        .ok_or_else(|| anyhow!("account #{} not found", account_id))?,
                );
                ACCOUNT_INFO_CACHE
                    .insert(account_id, account_info.clone())
                    .await;
                Ok(account_info)
            }
        }
    }

    async fn get_cached_tank_snapshots(
        api: &WargamingApi,
        account_id: i32,
    ) -> crate::Result<Arc<Vec<TankSnapshot>>> {
        match ACCOUNT_TANKS_CACHE.get(&account_id) {
            Some(snapshots) => {
                log::debug!("Cache hit on account #{} tanks.", account_id);
                Ok(snapshots)
            }
            None => {
                let snapshots = Arc::new(api.get_merged_tanks(account_id).await?);
                ACCOUNT_TANKS_CACHE
                    .insert(account_id, snapshots.clone())
                    .await;
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

    fn sort_vehicles(rows: &mut Vec<DisplayRow>, sort_by: SortBy) {
        match sort_by {
            SortBy::Battles => rows.sort_unstable_by_key(|row| -row.all_statistics.battles),
            SortBy::Wins => rows.sort_unstable_by_key(|row| -row.all_statistics.wins),
            SortBy::Nation => rows.sort_unstable_by_key(|row| row.vehicle.nation),
            SortBy::DamageDealt => {
                rows.sort_unstable_by_key(|row| -row.all_statistics.damage_dealt)
            }
            SortBy::DamagePerBattle => rows.sort_unstable_by(|left, right| {
                right
                    .damage_per_battle
                    .partial_cmp(&left.damage_per_battle)
                    .unwrap_or(Ordering::Equal)
            }),
            SortBy::Tier => rows.sort_unstable_by_key(|row| -row.vehicle.tier),
            SortBy::VehicleType => rows.sort_unstable_by_key(|row| row.vehicle.type_),
            SortBy::WinRate => rows.sort_unstable_by(|left, right| {
                right
                    .win_rate
                    .partial_cmp(&left.win_rate)
                    .unwrap_or(Ordering::Equal)
            }),
            SortBy::TrueWinRate => rows.sort_unstable_by(|left, right| {
                right
                    .true_win_rate
                    .0
                    .partial_cmp(&left.true_win_rate.0)
                    .unwrap_or(Ordering::Equal)
            }),
            SortBy::Gold => rows.sort_unstable_by(|left, right| {
                right
                    .gold_per_battle
                    .partial_cmp(&left.gold_per_battle)
                    .unwrap_or(Ordering::Equal)
            }),
            SortBy::TrueGold => rows.sort_unstable_by(|left, right| {
                right
                    .true_gold_per_battle
                    .partial_cmp(&left.true_gold_per_battle)
                    .unwrap_or(Ordering::Equal)
            }),
            SortBy::SurvivedBattles => {
                rows.sort_unstable_by_key(|row| -row.all_statistics.survived_battles)
            }
            SortBy::SurvivalRate => rows.sort_unstable_by(|left, right| {
                right
                    .survival_rate
                    .partial_cmp(&left.survival_rate)
                    .unwrap_or(Ordering::Equal)
            }),
        }
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

pub struct DisplayRow {
    pub vehicle: Arc<Vehicle>,
    pub all_statistics: AllStatistics,
    pub win_rate: f64,
    pub true_win_rate: (f64, f64),
    pub damage_per_battle: f64,
    pub survival_rate: f64,
    pub gold_per_battle: f64,
    pub true_gold_per_battle: f64,
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SortBy {
    Battles,
    Tier,
    Nation,
    VehicleType,
    Wins,
    WinRate,
    TrueWinRate,
    Gold,
    TrueGold,
    DamageDealt,
    DamagePerBattle,
    SurvivedBattles,
    SurvivalRate,
}
