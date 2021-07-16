use std::borrow::Cow;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use humantime::parse_duration;
use log::Level;
use ordered_float::OrderedFloat;
use smallvec::SmallVec;
use sqlx::PgPool;

use crate::database;
use crate::logging::set_user;
use crate::metrics::Stopwatch;
use crate::models::{subtract_tanks, AllStatistics, Tank, Vehicle};
use crate::statistics::wilson_score_interval;
use crate::tankopedia::get_vehicle;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::cache::account::tanks::AccountTanksCache;

pub struct ViewModel {
    pub account_id: i32,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub last_battle_time: DateTime<Utc>,
    pub has_recently_played: bool,
    pub is_active: bool,
    pub total_battles: i32,
    pub sort: Cow<'static, str>,
    pub period: StdDuration,
    pub warn_no_previous_account_info: bool,
    pub statistics: AllStatistics,
    pub rows: Vec<DisplayRow>,
    pub before: DateTime<Utc>,
}

impl ViewModel {
    pub async fn new(
        database: &PgPool,
        account_id: i32,
        sort: Option<String>,
        period: Option<String>,
        account_info_cache: &AccountInfoCache,
        account_tanks_cache: &AccountTanksCache,
    ) -> crate::Result<ViewModel> {
        let sort = sort
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(SORT_BY_BATTLES));
        let period = match period {
            Some(period) => parse_duration(&period)?,
            None => StdDuration::from_secs(43200),
        };
        log::info!("GET #{} within {:?}.", account_id, period);
        let _stopwatch =
            Stopwatch::new(format!("Done #{} within {:?}", account_id, period)).level(Level::Info);

        let current_info = account_info_cache.get(account_id).await?;
        set_user(&current_info.general.nickname);
        database::insert_account_or_ignore(database, &current_info.general).await?;

        let before = Utc::now() - Duration::from_std(period)?;
        let previous_info =
            database::retrieve_latest_account_snapshot(database, account_id, &before).await?;
        let current_tanks = account_tanks_cache.get(&current_info).await?;
        let tanks_delta = match &previous_info {
            Some(previous_info) => {
                let played_tank_ids: SmallVec<[i32; 96]> = current_tanks
                    .iter()
                    .filter(|(_, tank)| {
                        tank.last_battle_time > previous_info.general.last_battle_time
                    })
                    .map(|(tank_id, _)| *tank_id)
                    .collect();
                let previous_tank_snapshots = database::retrieve_latest_tank_snapshots(
                    database,
                    account_id,
                    &before,
                    &played_tank_ids,
                )
                .await?;
                subtract_tanks(&played_tank_ids, &current_tanks, &previous_tank_snapshots)
            }
            None => current_tanks.values().cloned().collect(), // FIXME: `cloned`.
        };

        let mut rows: Vec<DisplayRow> = tanks_delta
            .into_iter()
            .map(Self::make_display_row)
            .collect();
        Self::sort_tanks(&mut rows, &sort);

        let statistics = match &previous_info {
            Some(previous_info) => &current_info.statistics.all - &previous_info.statistics.all,
            None => current_info.statistics.all,
        };

        Ok(Self {
            account_id: current_info.general.id,
            nickname: current_info.general.nickname.clone(),
            created_at: current_info.general.created_at,
            last_battle_time: current_info.general.last_battle_time,
            total_battles: current_info.statistics.all.battles,
            has_recently_played: current_info.general.last_battle_time
                > (Utc::now() - Duration::hours(1)),
            is_active: current_info.is_active(),
            warn_no_previous_account_info: previous_info.is_none(),
            statistics,
            rows,
            before,
            period,
            sort,
        })
    }

    fn make_display_row(tank: Tank) -> DisplayRow {
        let vehicle = get_vehicle(tank.tank_id);
        let stats = &tank.all_statistics;
        let win_rate = stats.wins as f64 / stats.battles as f64;
        let expected_win_rate = wilson_score_interval(stats.battles, stats.wins);
        DisplayRow {
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
        }
    }

    fn sort_tanks(rows: &mut Vec<DisplayRow>, sort_by: &str) {
        match sort_by {
            SORT_BY_BATTLES => rows.sort_unstable_by_key(|row| -row.all_statistics.battles),
            SORT_BY_WINS => rows.sort_unstable_by_key(|row| -row.all_statistics.wins),
            SORT_BY_NATION => rows.sort_unstable_by_key(|row| row.vehicle.nation),
            SORT_BY_DAMAGE_DEALT => {
                rows.sort_unstable_by_key(|row| -row.all_statistics.damage_dealt)
            }
            SORT_BY_DAMAGE_PER_BATTLE => rows.sort_unstable_by_key(|row| -row.damage_per_battle),
            SORT_BY_TIER => rows.sort_unstable_by_key(|row| -row.vehicle.tier),
            SORT_BY_VEHICLE_TYPE => rows.sort_unstable_by_key(|row| row.vehicle.type_),
            SORT_BY_WIN_RATE => rows.sort_unstable_by_key(|row| -row.win_rate),
            SORT_BY_TRUE_WIN_RATE => rows.sort_unstable_by_key(|row| -row.expected_win_rate),
            SORT_BY_GOLD => rows.sort_unstable_by_key(|row| -row.gold_per_battle),
            SORT_BY_TRUE_GOLD => rows.sort_unstable_by_key(|row| -row.expected_gold_per_battle),
            SORT_BY_SURVIVED_BATTLES => {
                rows.sort_unstable_by_key(|row| -row.all_statistics.survived_battles)
            }
            SORT_BY_SURVIVAL_RATE => rows.sort_unstable_by_key(|row| -row.survival_rate),
            _ => {}
        }
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

pub const SORT_BY_BATTLES: &str = "battles";
pub const SORT_BY_TIER: &str = "tier";
pub const SORT_BY_NATION: &str = "nation";
pub const SORT_BY_VEHICLE_TYPE: &str = "vehicle-type";
pub const SORT_BY_WINS: &str = "wins";
pub const SORT_BY_WIN_RATE: &str = "win-rate";
pub const SORT_BY_TRUE_WIN_RATE: &str = "true-win-rate";
pub const SORT_BY_GOLD: &str = "gold";
pub const SORT_BY_TRUE_GOLD: &str = "true-gold";
pub const SORT_BY_DAMAGE_DEALT: &str = "damage-dealt";
pub const SORT_BY_DAMAGE_PER_BATTLE: &str = "damage-per-battle";
pub const SORT_BY_SURVIVED_BATTLES: &str = "survived-battles";
pub const SORT_BY_SURVIVAL_RATE: &str = "survival-rate";
