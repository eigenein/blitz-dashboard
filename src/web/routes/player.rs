use std::time::Instant;

use chrono::{Duration, Utc};
use chrono_humanize::Tense;
use humantime::parse_duration;
use maud::{html, PreEscaped, DOCTYPE};
use redis::aio::MultiplexedConnection;
use rocket::{uri, State};
use sqlx::PgPool;
use tokio::spawn;
use tokio::task::spawn_blocking;

use partials::*;

use crate::database::{insert_account_if_not_exists, retrieve_latest_tank_snapshots};
use crate::helpers::{format_elapsed, from_days, from_months};
use crate::logging::set_user;
use crate::math::statistics::ConfidenceInterval;
use crate::models::{subtract_tanks, Statistics, Tank};
use crate::tankopedia::remap_tank_id;
use crate::trainer::math::predict_win_rate;
use crate::trainer::{get_account_factors, get_all_vehicle_factors};
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::cache::account::tanks::AccountTanksCache;
use crate::web::partials::{
    account_search, datetime, footer, headers, home_button, icon_text, render_f64,
};
use crate::web::response::CustomResponse;
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
    let start_instant = Instant::now();
    let period = match period {
        Some(period) => match parse_duration(&period) {
            Ok(period) => period,
            Err(_) => return Ok(CustomResponse::BadRequest),
        },
        None => from_days(1),
    };

    let current_info = match account_info_cache.get(account_id).await? {
        Some(info) => info,
        None => return Ok(CustomResponse::NotFound),
    };
    set_user(&current_info.nickname);
    let insert_account_future = {
        let database = PgPool::clone(database);
        spawn(async move { insert_account_if_not_exists(&database, account_id).await })
    };
    tracing::info!(
        account_id = account_id,
        elapsed = format_elapsed(&start_instant).as_str(),
        "account ready",
    );

    let tanks = account_tanks_cache
        .get(current_info.base.id, current_info.base.last_battle_time)
        .await?;
    let tanks_delta = {
        let before = Utc::now() - Duration::from_std(period)?;
        let tank_ids: Vec<i32> = tanks.iter().map(Tank::tank_id).collect();
        let old_tank_snapshots =
            retrieve_latest_tank_snapshots(database, account_id, &tank_ids, &before).await?;
        spawn_blocking(move || subtract_tanks(tanks, old_tank_snapshots)).await?
    };
    tracing::info!(
        account_id = account_id,
        elapsed = format_elapsed(&start_instant).as_str(),
        "tanks delta ready",
    );
    let stats_delta: Statistics = tanks_delta.iter().map(|tank| tank.statistics.all).sum();
    let battle_life_time: i64 = tanks_delta
        .iter()
        .map(|tank| tank.statistics.battle_life_time.num_seconds())
        .sum();
    let current_win_rate = ConfidenceInterval::default_wilson_score_interval(
        current_info.statistics.n_battles(),
        current_info.statistics.n_wins(),
    );

    let mut redis = MultiplexedConnection::clone(redis);
    let account_factors = get_account_factors(&mut redis, account_id).await?;
    let vehicles_factors = get_all_vehicle_factors(&mut redis).await?;

    tracing::info!(
        account_id = account_id,
        elapsed = format_elapsed(&start_instant).as_str(),
        "model ready",
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
                }
                div.navbar-menu {
                    div.navbar-start {
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
            th {
                a data-sort="tier" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Уровень" }
                    }
                }
            }
            th { "Тип" }
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
                        span { "Общий ущерб" }
                    }
                }
            }
            th {
                a data-sort="damage-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Ущерб за бой" }
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
                script type="module" src="/static/table.js?v5" {};
            }
            body {
                (tracking_code.0)

                (navbar)

                section.section {
                    h1.title.is-hidden-desktop."is-4" {
                        span.icon-text {
                            span.icon { i.icon { i.fas.fa-user {} } }
                            span { (current_info.nickname) }
                        }
                    }

                    (tabs)

                    div.container {
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

                            @if stats_delta.battles != 0 {
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
                            }

                            @if stats_delta.battles != 0 {
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
                        }

                        div.tile.is-ancestor {
                            @if stats_delta.battles != 0 {
                                div.tile."is-4".is-parent {
                                    @let period_win_rate = stats_delta.true_win_rate();
                                    div.tile.is-child.card.(partial_cmp_class(period_win_rate.partial_cmp(&current_win_rate))) {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-percentage", "Процент побед")) }
                                        }
                                        div.card-content {
                                            div.level {
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
                            }

                            @if stats_delta.battles != 0 {
                                div.tile."is-2".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-check", "Победы")) }
                                        }
                                        div.card-content {
                                            div.level {
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
                            }

                            @if stats_delta.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-heart", "Выживаемость")) }
                                        }
                                        div.card-content {
                                            div.level {
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
                            }

                            @if stats_delta.shots != 0 {
                                div.tile."is-2".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-bullseye", "Попадания")) }
                                        }
                                        div.card-content {
                                            div.level {
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

                        @if !tanks_delta.is_empty() {
                            div.box {
                                div.table-container {
                                    table.table.is-hoverable.is-striped.is-fullwidth id="vehicles" {
                                        thead { (vehicles_thead) }
                                        tbody {
                                            @for tank in &tanks_delta {
                                                @let predicted_win_rate = account_factors.as_ref().and_then(|account_factors| {
                                                    let tank_id = remap_tank_id(tank.statistics.base.tank_id);
                                                    vehicles_factors.get(&tank_id).map(|vehicle_factors| {
                                                        predict_win_rate(vehicle_factors, account_factors).clamp(0.0, 1.0)
                                                    })
                                                });
                                                (render_tank_tr(tank, &current_win_rate, predicted_win_rate))
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

                (PreEscaped("<!-- Account factors: "))
                (format!("{:?}", account_factors))
                (PreEscaped(" -->"))
            }
        }
    };

    let result = Ok(CustomResponse::CachedHtml(
        "max-age=60, stale-while-revalidate=3600",
        markup,
    ));
    insert_account_future.await??;
    tracing::info!(
        account_id = account_id,
        elapsed = format_elapsed(&start_instant).as_str(),
        "finished",
    );
    result
}
