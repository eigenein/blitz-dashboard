use std::cmp::Ordering;
use std::time::Instant;

use chrono::{Duration, Utc};
use chrono_humanize::Tense;
use futures::future::try_join3;
use humantime::parse_duration;
use indexmap::IndexMap;
use maud::{html, PreEscaped, DOCTYPE};
use redis::aio::MultiplexedConnection;
use rocket::{uri, State};
use sqlx::PgPool;
use tokio::task::spawn_blocking;

use crate::database::{insert_account_if_not_exists, retrieve_latest_tank_snapshots};
use crate::helpers::{format_elapsed, from_days, from_hours, from_months};
use crate::logging::set_user;
use crate::math::statistics::ConfidenceInterval;
use crate::models::{subtract_tanks, Statistics, Tank};
use crate::tankopedia::remap_tank_id;
use crate::trainer::math::predict_probability;
use crate::trainer::model::{get_account_factors, get_all_vehicle_factors};
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::cache::account::tanks::AccountTanksCache;
use crate::web::partials::*;
use crate::web::response::CustomResponse;
use crate::web::routes::player::partials::*;
use crate::web::TrackingCode;

pub mod partials;

#[tracing::instrument(skip_all, fields(account_id = account_id, period = period.as_deref()))]
#[rocket::get("/ru/<account_id>?<period>")]
pub async fn get(
    account_id: i32,
    period: Option<String>,
    database: &State<PgPool>,
    account_info_cache: &State<AccountInfoCache>,
    tracking_code: &State<TrackingCode>,
    account_tanks_cache: &State<AccountTanksCache>,
    redis: &State<MultiplexedConnection>,
) -> crate::web::result::Result<CustomResponse> {
    let mut redis = (*redis).clone();

    let start_instant = Instant::now();
    let period = match period {
        Some(period) => match parse_duration(&period) {
            Ok(period) => period,
            Err(_) => return Ok(CustomResponse::BadRequest),
        },
        None => from_days(1),
    };

    let (current_info, tanks, old_tank_snapshots) = {
        let before = Utc::now() - Duration::from_std(period)?;
        try_join3(
            account_info_cache.get(account_id),
            account_tanks_cache.get(account_id),
            retrieve_latest_tank_snapshots(database, account_id, &before),
        )
        .await?
    };
    let current_info = match current_info {
        Some(info) => info,
        None => return Ok(CustomResponse::NotFound),
    };
    set_user(&current_info.nickname);
    let old_info = insert_account_if_not_exists(database, account_id).await?;

    let predictions = make_predictions(&mut redis, account_id, &tanks).await?;
    let tanks_delta = { spawn_blocking(move || subtract_tanks(tanks, old_tank_snapshots)).await? };
    let stats_delta: Statistics = tanks_delta.iter().map(|tank| tank.statistics.all).sum();
    let battle_life_time: i64 = tanks_delta
        .iter()
        .map(|tank| tank.statistics.battle_life_time.num_seconds())
        .sum();
    let current_win_rate = ConfidenceInterval::default_wilson_score_interval(
        current_info.statistics.n_battles(),
        current_info.statistics.n_wins(),
    );

    let navbar = html! {
        nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
            div.container {
                div.navbar-brand {
                    (home_button())

                    div.navbar-item title="Последний бой" {
                        span.icon-text.(if current_info.has_recently_played() { "has-text-success-dark" } else if !current_info.is_active() { "has-text-danger-dark" } else { "" }) {
                            span.icon { i.fas.fa-bullseye {} }
                            time
                                datetime=(current_info.base.last_battle_time.to_rfc3339())
                                title=(current_info.base.last_battle_time) {
                                    (datetime(current_info.base.last_battle_time, Tense::Past))
                                }
                        }
                    }

                    div.navbar-item title="Боев" {
                        span.icon-text {
                            span.icon { i.fas.fa-sort-numeric-up-alt {} }
                            span { (current_info.statistics.n_battles()) }
                        }
                    }

                    div.navbar-item title="Возраст аккаунта" {
                        span.icon-text {
                            @if current_info.is_account_birthday() {
                                span.icon title="День рождения!" { i.fas.fa-birthday-cake.has-text-danger {} }
                            } @else {
                                span.icon { i.far.fa-calendar-alt {} }
                            }
                            span title=(current_info.created_at) {
                                (datetime(current_info.created_at, Tense::Present))
                            }
                        }
                    }
                }
                div.navbar-menu.is-active {
                    div.navbar-end {
                        form.navbar-item action="/search" method="GET" {
                            (account_search("", &current_info.nickname, false, current_info.is_prerelease_account()))
                        }
                    }
                }
            }
        }
    };
    let tabs = html! {
        nav.tabs.is-boxed {
            div.container {
                ul {
                    (render_period_li(period, from_hours(12), "12 часов"))
                    (render_period_li(period, from_days(1), "24 часа"))
                    (render_period_li(period, from_days(2), "2 дня"))
                    (render_period_li(period, from_days(3), "3 дня"))
                    (render_period_li(period, from_days(7), "Неделя"))
                    (render_period_li(period, from_days(14), "2 недели"))
                    (render_period_li(period, from_days(21), "3 недели"))
                    (render_period_li(period, from_months(1), "Месяц"))
                    (render_period_li(period, from_months(2), "2 месяца"))
                    (render_period_li(period, from_months(3), "3 месяца"))
                }
            }
        }
    };
    let vehicles_thead = html! {
        tr {
            th {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-truck-monster {} }
                    span { "Техника" }
                }
            }
            th { "Тип" }
            th {
                a data-sort="battle-life-time" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Время в боях" }
                    }
                }
            }
            th {
                a data-sort="battles" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Бои" }
                    }
                }
            }
            th {
                a data-sort="wins" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Победы" }
                    }
                }
            }
            th {
                a data-sort="win-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Текущий процент побед" { "WR" } }
                    }
                }
            }
            th {
                a data-sort="true-win-rate-mean" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Истинный процент побед – это WR, скорректированный на число боев" { "TWR" } }
                    }
                }
            }
            th.is-white-space-nowrap {
                sup title="В разработке" { strong.has-text-danger-dark { "ɑ" } }
                a data-sort="predicted-win-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Предсказываемая вероятность победы этого игрока на этом танке прямо сейчас" { "PWP" } }
                    }
                }
            }
            th {
                a data-sort="frags-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Среднее число фрагов за бой" { "FPB" } }
                    }
                }
            }
            th {
                a data-sort="wins-per-hour" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Число побед за время жизни танка в бою – полезно для событий на победы" { "WPH" } }
                    }
                }
            }
            th {
                a data-sort="expected-wins-per-hour" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Число побед в час, скорректированное на число проведенных боев" { "TWPH" } }
                    }
                }
            }
            th {
                a data-sort="gold" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Текущий доход от золотых бустеров, если они были установлены" { "Золото" } }
                    }
                }
            }
            th {
                a data-sort="true-gold" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Доходность золотого бустера за бой, скорректированная на число проведенных боев" { "Истинное золото" } }
                    }
                }
            }
            th {
                a data-sort="damage-dealt" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Общий урон" }
                    }
                }
            }
            th.is-white-space-nowrap {
                a data-sort="damage-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Урон за бой" }
                    }
                }
            }
            th {
                a data-sort="survived-battles" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Выжил" }
                    }
                }
            }
            th {
                a data-sort="survival-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Выживаемость" }
                    }
                }
            }
        }
    };
    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                link rel="canonical" href=(uri!(get(account_id = account_id, period = _)));
                title { (current_info.nickname) " – Я статист!" }
            }
            body {
                (tracking_code.0)

                (navbar)

                section.section {
                    (tabs)

                    div.container {
                        @if stats_delta.battles != 0 {
                            div.columns.is-multiline {
                                div.column."is-6-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-sort-numeric-up-alt", "Бои")) }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Победы" }
                                                        p.title { (stats_delta.wins) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Выжил" }
                                                        p.title { (stats_delta.survived_battles) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-6-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-house-damage", "Нанесенный урон")) }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.damage_dealt) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_f64(stats_delta.damage_per_battle(), 0)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-12-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-skull-crossbones", "Уничтоженная техника")) }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.frags) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_f64(stats_delta.frags_per_battle(), 1)) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "В час" }
                                                        p.title { (render_f64(stats_delta.frags as f64 / battle_life_time as f64 * 3600.0, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div.columns.is-multiline {
                                div.column."is-8-tablet"."is-4-desktop" {
                                    @let period_win_rate = stats_delta.true_win_rate();
                                    div.card.(partial_cmp_class(period_win_rate.partial_cmp(&current_win_rate))) {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-percentage", "Процент побед")) }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Средний" }
                                                        p.title { (render_percentage(stats_delta.current_win_rate())) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Истинный" }
                                                        p.title.is-white-space-nowrap {
                                                            (render_percentage(period_win_rate.mean))
                                                            span.has-text-grey-light { " ±" (render_f64(100.0 * period_win_rate.margin, 1)) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-4-tablet"."is-2-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-check", "Победы")) }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "В час" }
                                                        p.title { (render_f64(stats_delta.wins as f64 / battle_life_time as f64 * 3600.0, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-8-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-heart", "Выживаемость")) }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Средняя" }
                                                        p.title { (render_percentage(stats_delta.survival_rate())) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Истинная" }
                                                        p.title.is-white-space-nowrap {
                                                            @let expected_period_survival_rate = ConfidenceInterval::default_wilson_score_interval(stats_delta.battles, stats_delta.survived_battles);
                                                            (render_percentage(expected_period_survival_rate.mean))
                                                            span.has-text-grey-light { (format!(" ±{:.1}", 100.0 * expected_period_survival_rate.margin)) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                @if stats_delta.shots != 0 {
                                    div.column."is-4-tablet"."is-2-desktop" {
                                        div.card {
                                            header.card-header {
                                                p.card-header-title { (icon_text("fas fa-bullseye", "Попадания")) }
                                            }
                                            div.card-content {
                                                div.level.is-mobile {
                                                    div.level-item.has-text-centered {
                                                        div {
                                                            p.heading { "В среднем" }
                                                            p.title { (render_percentage(stats_delta.hit_rate())) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } @else {
                            article.message.is-warning {
                                div.message-body {
                                    p { "Пользователь не играл в случайных боях за этот период времени." }
                                    p { "Последний бой закончился " strong { (datetime(current_info.base.last_battle_time, Tense::Present)) } " назад." }
                                }
                            }
                        }

                        @if !tanks_delta.is_empty() {
                            div.box {
                                div.table-container {
                                    table.table.is-hoverable.is-striped.is-fullwidth id="vehicles" {
                                        thead { (vehicles_thead) }
                                        tbody {
                                            @for tank in &tanks_delta {
                                                @let predicted_win_rate = predictions.get(&tank.statistics.base.tank_id).copied();
                                                (render_tank_tr(tank, &current_win_rate, predicted_win_rate, old_info.last_battle_time)?)
                                            }
                                        }
                                        @if tanks_delta.len() >= 25 {
                                            tfoot { (vehicles_thead) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                (footer())

                script type="module" {
                    (PreEscaped(r#"""
                        "use strict";
                        
                        import { initSortableTable } from "/static/table.js?v5";
                        
                        (function () {
                            const vehicles = document.getElementById("vehicles");
                            if (vehicles != null) {
                                initSortableTable(vehicles, "battles");
                            }
                        })();
                    """#))
                }
            }
        }
    };

    let result = Ok(CustomResponse::CachedHtml(
        "max-age=60, stale-while-revalidate=3600",
        markup,
    ));
    tracing::info!(
        account_id = account_id,
        elapsed = format_elapsed(&start_instant).as_str(),
        "finished",
    );
    result
}

/// Generate win rate predictions for the account's tanks.
///
/// Returns an ordered map of the win rates by tank ID's.
/// The entries are sorted by the predicted win rate in the descending order.
/// That way I'm able to quickly select top N tanks by the predicted win rate.
async fn make_predictions(
    redis: &mut MultiplexedConnection,
    account_id: i32,
    tanks: &[Tank],
) -> crate::Result<IndexMap<i32, f64>> {
    let account_factors = match get_account_factors(redis, account_id).await? {
        Some(factors) => factors,
        None => return Ok(IndexMap::new()),
    };
    let vehicles_factors = get_all_vehicle_factors(redis).await?;

    let mut predictions: IndexMap<i32, f64> = tanks
        .iter()
        .map(|tank| remap_tank_id(tank.statistics.base.tank_id))
        .filter_map(|tank_id| {
            vehicles_factors.get(&tank_id).map(|vehicle_factors| {
                (
                    tank_id,
                    predict_probability(vehicle_factors, &account_factors),
                )
            })
        })
        .collect();
    predictions.sort_by(|_, left, _, right| right.partial_cmp(left).unwrap_or(Ordering::Equal));
    Ok(predictions)
}
