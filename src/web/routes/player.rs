use std::borrow::Cow;
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use chrono_humanize::Tense;
use humantime::parse_duration;
use log::Level;
use maud::{html, DOCTYPE};
use rocket::response::content::Html;
use rocket::State;
use smallvec::SmallVec;
use sqlx::PgPool;

use partials::*;

use crate::database;
use crate::logging::set_user;
use crate::metrics::Stopwatch;
use crate::models::subtract_tanks;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::cache::account::tanks::AccountTanksCache;
use crate::web::partials::{account_search, datetime, footer, headers, icon_text};
use crate::web::TrackingCode;

pub mod partials;

#[rocket::get("/ru/<account_id>?<sort>&<period>")]
pub async fn get(
    account_id: i32,
    sort: Option<String>,
    period: Option<String>,
    database: &State<PgPool>,
    account_info_cache: &State<AccountInfoCache>,
    tracking_code: &State<TrackingCode>,
    account_tanks_cache: &State<AccountTanksCache>,
) -> crate::web::result::Result<Html<String>> {
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
                .filter(|(_, tank)| tank.last_battle_time > previous_info.general.last_battle_time)
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

        // FIXME: `cloned`, after https://github.com/eigenein/blitz-dashboard/issues/74.
        None => current_tanks.values().cloned().collect(),
    };

    let mut rows: Vec<DisplayRow> = tanks_delta.into_iter().map(make_display_row).collect();
    sort_tanks(&mut rows, &sort);

    let statistics = match &previous_info {
        Some(previous_info) => &current_info.statistics.all - &previous_info.statistics.all,
        None => current_info.statistics.all,
    };

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                title { (current_info.general.nickname) " – Я статист!" }
            }
            body {
                (tracking_code.0)
                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.container {
                        div.navbar-brand {
                            div.navbar-item {
                                div.buttons {
                                    a.button.is-link.is-rounded href="/" {
                                        span.icon { i.fas.fa-home {} }
                                        span { "На главную" }
                                    }
                                }
                            }
                        }
                        div.navbar-menu {
                            div.navbar-end {
                                form.navbar-item action="/search" method="GET" {
                                    (account_search("", &current_info.general.nickname, false))
                                }
                            }
                        }
                    }
                }

                section.section {
                    div.container {
                        div.tile.is-ancestor {
                            div.tile."is-6".is-parent {
                                div.tile.is-child.card {
                                    header.card-header {
                                        p.card-header-title { (icon_text("fas fa-user", &current_info.general.nickname)) }
                                    }
                                    div.card-content {
                                        div.level {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Возраст" }
                                                    p.title title=(current_info.general.created_at) {
                                                        (datetime(current_info.general.created_at, Tense::Present))
                                                    }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Боев" }
                                                    p.title { (current_info.statistics.all.battles) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Последний бой" }
                                                    p.title.(if current_info.has_recently_played() { "has-text-success" } else if !current_info.is_active() { "has-text-danger" } else { "" }) {
                                                        time
                                                            datetime=(current_info.general.last_battle_time.to_rfc3339())
                                                            title=(current_info.general.last_battle_time) {
                                                                (datetime(current_info.general.last_battle_time, Tense::Past))
                                                            }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        #period.tabs.is-boxed {
                            ul {
                                (render_period_li(&sort, period, StdDuration::from_secs(3600), "Час"))
                                (render_period_li(&sort, period, StdDuration::from_secs(2 * 3600), "2 часа"))
                                (render_period_li(&sort, period, StdDuration::from_secs(4 * 3600), "4 часа"))
                                (render_period_li(&sort, period, StdDuration::from_secs(8 * 3600), "8 часов"))
                                (render_period_li(&sort, period, StdDuration::from_secs(12 * 3600), "12 часов"))
                                (render_period_li(&sort, period, StdDuration::from_secs(86400), "24 часа"))
                                (render_period_li(&sort, period, StdDuration::from_secs(2 * 86400), "2 дня"))
                                (render_period_li(&sort, period, StdDuration::from_secs(3 * 86400), "3 дня"))
                                (render_period_li(&sort, period, StdDuration::from_secs(7 * 86400), "Неделя"))
                                (render_period_li(&sort, period, StdDuration::from_secs(2630016), "Месяц"))
                                (render_period_li(&sort, period, StdDuration::from_secs(2 * 2630016), "2 месяца"))
                                (render_period_li(&sort, period, StdDuration::from_secs(3 * 2630016), "3 месяца"))
                                (render_period_li(&sort, period, StdDuration::from_secs(31557600), "Год"))
                            }
                        }

                        @if previous_info.is_none() {
                            article.message.is-warning {
                                div.message-body {
                                    strong { "Отображается статистика за все время." }
                                    " У нас нет сведений об аккаунте за этот период."
                                }
                            }
                        }

                        @if current_info.general.last_battle_time >= before && statistics.battles == 0 {
                            article.message.is-warning {
                                div.message-body {
                                    strong { "Нет случайных боев за этот период." }
                                    " Вероятно, игрок проводил время в других режимах."
                                }
                            }
                        }

                        div.tile.is-ancestor {
                            div.tile."is-4".is-parent {
                                div.tile.is-child.card {
                                    header.card-header {
                                        p.card-header-title { (icon_text("fas fa-sort-numeric-up-alt", "Бои")) }
                                    }
                                    div.card-content {
                                        div.level {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Всего" }
                                                    p.title { (statistics.battles) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Победы" }
                                                    p.title { (statistics.wins) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Выжил" }
                                                    p.title { (statistics.survived_battles) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            @if statistics.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-house-damage", "Нанесенный ущерб")) }
                                        }
                                        div.card-content {
                                            div.level {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (statistics.damage_dealt) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_f64(statistics.damage_dealt as f64 / statistics.battles as f64, 0)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            @if statistics.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-skull-crossbones", "Уничтоженная техника")) }
                                        }
                                        div.card-content {
                                            div.level {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (statistics.frags) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_f64(statistics.frags as f64 / statistics.battles as f64, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div.tile.is-ancestor {
                            @if statistics.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-percentage", "Победы")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(statistics.battles, statistics.wins))
                                        }
                                    }
                                }
                            }

                            @if statistics.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-heart", "Выживаемость")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(statistics.battles, statistics.survived_battles))
                                        }
                                    }
                                }
                            }

                            @if statistics.shots != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-bullseye", "Попадания")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(statistics.shots, statistics.hits))
                                        }
                                    }
                                }
                            }
                        }

                        @if !rows.is_empty() {
                            div.box {
                                div.table-container {
                                    table#vehicles.table.is-hoverable.is-striped.is-fullwidth {
                                        thead {
                                            tr {
                                                th { "Техника" }
                                                (render_vehicles_th(&sort, period, SORT_BY_TIER, html! { "Уровень" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_NATION, html! { "Нация" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_VEHICLE_TYPE, html! { "Тип" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_BATTLES, html! { "Бои" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_WINS, html! { "Победы" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_WIN_RATE, html! { "Текущий процент побед" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_TRUE_WIN_RATE, html! { "Ожидаемый процент побед" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_GOLD, html! { abbr title="Текущий доход от золотых бустеров за бой, если они были установлены" { "Заработанное золото" } }))
                                                (render_vehicles_th(&sort, period, SORT_BY_TRUE_GOLD, html! { abbr title="Средняя ожидаемая доходность золотого бустера за бой" { "Ожидаемое золото" } }))
                                                (render_vehicles_th(&sort, period, SORT_BY_DAMAGE_DEALT, html! { "Ущерб" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_DAMAGE_PER_BATTLE, html! { "Ущерб за бой" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_SURVIVED_BATTLES, html! { "Выжил" }))
                                                (render_vehicles_th(&sort, period, SORT_BY_SURVIVAL_RATE, html! { "Выживаемость" }))
                                            }
                                        }
                                        tbody {
                                            @for row in &rows {
                                                tr {
                                                    th.is-white-space-nowrap { (render_vehicle_name(&row.vehicle)) }
                                                    td.has-text-centered { strong { (render_tier(row.vehicle.tier)) } }
                                                    td.has-text-centered { (render_nation(&row.vehicle.nation)) }
                                                    td { (format!("{:?}", row.vehicle.type_)) }
                                                    td { (row.all_statistics.battles) }
                                                    td { (row.all_statistics.wins) }
                                                    td.has-text-info { strong { (render_f64(100.0 * row.win_rate.0, 1)) "%" } }
                                                    td.has-text-centered.is-white-space-nowrap {
                                                        strong { (render_f64(100.0 * row.expected_win_rate.0, 1)) "%" }
                                                        span.(margin_class(row.expected_win_rate_margin.0, 0.1, 0.25)) {
                                                            " ±" (render_f64(row.expected_win_rate_margin.0 * 100.0, 1))
                                                        }
                                                    }
                                                    td {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                                                            span { strong { (render_f64(row.gold_per_battle.0, 1)) } }
                                                        }
                                                    }
                                                    td.is-white-space-nowrap {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                                                            span {
                                                                strong { (render_f64(row.expected_gold_per_battle.0, 1)) }
                                                                @let gold_margin = row.vehicle.tier as f64 * row.expected_win_rate_margin.0;
                                                                span.(margin_class(gold_margin, 2.0, 3.0)) {
                                                                    " ±"
                                                                    (render_f64(gold_margin, 1))
                                                                }
                                                            }
                                                        }
                                                    }
                                                    td { (row.all_statistics.damage_dealt) }
                                                    td { (render_f64(row.damage_per_battle.0, 0)) }
                                                    td { (row.all_statistics.survived_battles) }
                                                    td {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span.icon { i.fas.fa-heart.has-text-danger {} }
                                                            span { (render_f64(100.0 * row.survival_rate.0, 0)) "%" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                (footer())
            }
        }
    };

    Ok(Html(markup.into_string()))
}

pub fn get_account_url(account_id: i32) -> String {
    format!("/ru/{}", account_id)
}

// TODO: https://github.com/eigenein/blitz-dashboard/issues/74.
const SORT_BY_BATTLES: &str = "battles";
const SORT_BY_TIER: &str = "tier";
const SORT_BY_NATION: &str = "nation";
const SORT_BY_VEHICLE_TYPE: &str = "vehicle-type";
const SORT_BY_WINS: &str = "wins";
const SORT_BY_WIN_RATE: &str = "win-rate";
const SORT_BY_TRUE_WIN_RATE: &str = "true-win-rate";
const SORT_BY_GOLD: &str = "gold";
const SORT_BY_TRUE_GOLD: &str = "true-gold";
const SORT_BY_DAMAGE_DEALT: &str = "damage-dealt";
const SORT_BY_DAMAGE_PER_BATTLE: &str = "damage-per-battle";
const SORT_BY_SURVIVED_BATTLES: &str = "survived-battles";
const SORT_BY_SURVIVAL_RATE: &str = "survival-rate";

// TODO: https://github.com/eigenein/blitz-dashboard/issues/74.
fn sort_tanks(rows: &mut Vec<DisplayRow>, sort_by: &str) {
    match sort_by {
        SORT_BY_BATTLES => rows.sort_unstable_by_key(|row| -row.all_statistics.battles),
        SORT_BY_WINS => rows.sort_unstable_by_key(|row| -row.all_statistics.wins),
        SORT_BY_NATION => rows.sort_unstable_by_key(|row| row.vehicle.nation),
        SORT_BY_DAMAGE_DEALT => rows.sort_unstable_by_key(|row| -row.all_statistics.damage_dealt),
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
