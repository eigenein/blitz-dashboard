use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, Utc};
use itertools::{merge_join_by, EitherOrBoth};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use tide::Request;

use crate::database;
use crate::logging::set_user;
use crate::models::{AllStatistics, Tank, Vehicle};
use crate::statistics::wilson_score_interval;
use crate::web::state::State;

pub struct ViewModel {
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
    pub before: DateTime<Utc>,
}

impl ViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<ViewModel> {
        let account_id = Self::parse_account_id(&request)?;
        let query: Query = request
            .query()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?;
        log::info!(
            "Requested player #{} within {:?}.",
            account_id,
            query.period,
        );

        let state = request.state();
        let current_info = state.retrieve_account_info(account_id).await?;
        set_user(&current_info.nickname);
        database::insert_account_or_ignore(&state.database, &current_info.basic).await?;

        let current_tanks = state.retrieve_tanks(&current_info).await?;
        let total_tanks = current_tanks.len();
        let before = Utc::now() - Duration::from_std(query.period)?;
        let previous_info =
            database::retrieve_latest_account_snapshot(&state.database, account_id, &before)
                .await?;
        let previous_tanks = if previous_info.is_some() {
            database::retrieve_latest_tank_snapshots(&state.database, account_id, &before).await?
        } else {
            Vec::new()
        };

        let mut rows: Vec<DisplayRow> = Vec::new();
        for tank in
            Self::subtract_tank_snapshots(current_tanks.to_vec(), previous_tanks).into_iter()
        {
            rows.push(Self::make_display_row(
                state.get_vehicle(tank.tank_id).await?,
                tank,
            )?);
        }
        Self::sort_tanks(&mut rows, query.sort_by);

        let statistics = match &previous_info {
            Some(previous_info) => &current_info.statistics.all - &previous_info.statistics.all,
            None => current_info.statistics.all.clone(),
        };

        Ok(Self {
            account_id: current_info.basic.id,
            nickname: current_info.nickname.clone(),
            created_at: current_info.created_at,
            last_battle_time: current_info.basic.last_battle_time,
            total_battles: current_info.statistics.all.battles,
            has_recently_played: current_info.basic.last_battle_time
                > (Utc::now() - Duration::hours(1)),
            is_active: current_info.is_active(),
            warn_no_previous_account_info: previous_info.is_none(),
            query,
            statistics,
            rows,
            total_tanks,
            before,
        })
    }

    fn make_display_row(vehicle: Vehicle, tank: Tank) -> crate::Result<DisplayRow> {
        let stats = &tank.all_statistics;
        let win_rate = stats.wins as f64 / stats.battles as f64;
        let expected_win_rate = wilson_score_interval(stats.battles, stats.wins);
        Ok(DisplayRow {
            win_rate: OrderedFloat(win_rate),
            expected_win_rate: OrderedFloat(expected_win_rate.0),
            expected_win_rate_margin: OrderedFloat(expected_win_rate.1),
            damage_per_battle: OrderedFloat(stats.damage_dealt as f64 / stats.battles as f64),
            survival_rate: OrderedFloat(stats.survived_battles as f64 / stats.battles as f64),
            all_statistics: tank.all_statistics,
            gold_per_battle: OrderedFloat(10.0 + vehicle.tier as f64 * win_rate),
            expected_gold_per_battle: OrderedFloat(
                10.0 + vehicle.tier as f64 * expected_win_rate.0,
            ),
            vehicle,
        })
    }

    fn subtract_tank_snapshots(
        mut actual_snapshots: Vec<Tank>,
        mut previous_snapshots: Vec<Tank>,
    ) -> Vec<Tank> {
        actual_snapshots.sort_unstable_by_key(|snapshot| snapshot.tank_id);
        previous_snapshots.sort_unstable_by_key(|snapshot| snapshot.tank_id);

        merge_join_by(actual_snapshots, previous_snapshots, |left, right| {
            left.tank_id.cmp(&right.tank_id)
        })
        .filter_map(|item| match item {
            EitherOrBoth::Both(current, previous)
                if current.all_statistics.battles > previous.all_statistics.battles =>
            {
                Some(Tank {
                    account_id: current.account_id,
                    tank_id: current.tank_id,
                    all_statistics: &current.all_statistics - &previous.all_statistics,
                    last_battle_time: current.last_battle_time,
                    battle_life_time: current.battle_life_time - previous.battle_life_time,
                })
            }
            EitherOrBoth::Left(current) => Some(current),
            _ => None,
        })
        .collect::<Vec<Tank>>()
    }

    fn sort_tanks(rows: &mut Vec<DisplayRow>, sort_by: SortBy) {
        match sort_by {
            SortBy::Battles => rows.sort_unstable_by_key(|row| -row.all_statistics.battles),
            SortBy::Wins => rows.sort_unstable_by_key(|row| -row.all_statistics.wins),
            SortBy::Nation => rows.sort_unstable_by_key(|row| row.vehicle.nation),
            SortBy::DamageDealt => {
                rows.sort_unstable_by_key(|row| -row.all_statistics.damage_dealt)
            }
            SortBy::DamagePerBattle => rows.sort_unstable_by_key(|row| -row.damage_per_battle),
            SortBy::Tier => rows.sort_unstable_by_key(|row| -row.vehicle.tier),
            SortBy::VehicleType => rows.sort_unstable_by_key(|row| row.vehicle.type_),
            SortBy::WinRate => rows.sort_unstable_by_key(|row| -row.win_rate),
            SortBy::TrueWinRate => rows.sort_unstable_by_key(|row| -row.expected_win_rate),
            SortBy::Gold => rows.sort_unstable_by_key(|row| -row.gold_per_battle),
            SortBy::TrueGold => rows.sort_unstable_by_key(|row| -row.expected_gold_per_battle),
            SortBy::SurvivedBattles => {
                rows.sort_unstable_by_key(|row| -row.all_statistics.survived_battles)
            }
            SortBy::SurvivalRate => rows.sort_unstable_by_key(|row| -row.survival_rate),
        }
    }

    /// Parses account ID URL segment.
    fn parse_account_id(request: &Request<State>) -> crate::Result<i32> {
        request
            .param("account_id")
            .map_err(tide::Error::into_inner)
            .context("missing account ID")?
            .parse()
            .context("invalid account ID")
    }
}

pub struct DisplayRow {
    pub vehicle: Vehicle,
    pub all_statistics: AllStatistics,
    pub win_rate: OrderedFloat<f64>,
    pub expected_win_rate: OrderedFloat<f64>,
    pub expected_win_rate_margin: OrderedFloat<f64>,
    pub damage_per_battle: OrderedFloat<f64>,
    pub survival_rate: OrderedFloat<f64>,
    pub gold_per_battle: OrderedFloat<f64>,
    pub expected_gold_per_battle: OrderedFloat<f64>,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Query {
    #[serde(
        default = "Query::default_period",
        with = "humantime_serde",
        skip_serializing_if = "Query::skip_serializing_period_if"
    )]
    pub period: StdDuration,

    #[serde(
        default = "Query::default_sort_by",
        rename = "sort-by",
        skip_serializing_if = "Query::skip_serializing_sort_by_if"
    )]
    pub sort_by: SortBy,
}

impl Query {
    const DEFAULT_PERIOD: StdDuration = StdDuration::from_secs(86400);
    const DEFAULT_SORT_BY: SortBy = SortBy::Battles;

    fn default_period() -> StdDuration {
        Self::DEFAULT_PERIOD
    }

    fn skip_serializing_period_if(period: &StdDuration) -> bool {
        period == &Self::DEFAULT_PERIOD
    }

    fn default_sort_by() -> SortBy {
        Self::DEFAULT_SORT_BY
    }

    fn skip_serializing_sort_by_if(sort_by: &SortBy) -> bool {
        sort_by == &Self::DEFAULT_SORT_BY
    }
}

impl Query {
    pub fn period(&self, period: StdDuration) -> Self {
        Self {
            period,
            sort_by: self.sort_by,
        }
    }

    pub fn sort_by(&self, sort_by: SortBy) -> Self {
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
