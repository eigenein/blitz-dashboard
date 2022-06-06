//! Player view.
//!
//! «Abandon hope, all ye who enter here».

use std::time::Instant;

use chrono::{Duration, Utc};
use chrono_humanize::Tense;
use futures::future::try_join;
use humantime::{format_duration, parse_duration};
use itertools::Itertools;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use rocket::http::Status;
use rocket::{uri, State};
use sqlx::PgPool;
use tracing::{instrument, warn};

use crate::database::{insert_account_if_not_exists, retrieve_latest_tank_snapshots};
use crate::helpers::sentry::set_user;
use crate::helpers::time::{format_elapsed, from_days, from_months};
use crate::math::statistics::{ConfidenceInterval, Z};
use crate::prelude::*;
use crate::tankopedia::get_vehicle;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::models::{subtract_tanks, BasicStatistics, Tank, TankType};
use crate::web::partials::*;
use crate::web::response::CustomResponse;
use crate::web::views::player::partials::*;
use crate::web::TrackingCode;

pub mod partials;

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, level = "warn", fields(account_id = account_id, period = ?period))]
#[rocket::get("/ru/<account_id>?<period>")]
pub async fn get(
    account_id: i32,
    period: Option<String>,
    database: &State<PgPool>,
    info_cache: &State<AccountInfoCache>,
    tracking_code: &State<TrackingCode>,
    tanks_cache: &State<AccountTanksCache>,
) -> crate::web::result::Result<CustomResponse> {
    let start_instant = Instant::now();
    let period = match period {
        Some(period) => match parse_duration(&period) {
            Ok(period) => period,
            Err(_) => return Ok(CustomResponse::Status(Status::BadRequest)),
        },
        None => from_days(1),
    };

    let (current_info, tanks) =
        try_join(info_cache.get(account_id), tanks_cache.get(account_id)).await?;
    let current_info = match current_info {
        Some(info) => info,
        None => return Ok(CustomResponse::Status(Status::NotFound)),
    };
    set_user(&current_info.nickname);
    let old_tank_snapshots = {
        let before = Utc::now() - Duration::from_std(period)?;
        let tank_ids = tanks.iter().map(Tank::tank_id).collect_vec();
        retrieve_latest_tank_snapshots(database, account_id, before, &tank_ids).await?
    };
    insert_account_if_not_exists(database, account_id).await?;

    let tanks_delta = subtract_tanks(tanks, old_tank_snapshots);
    let stats_delta: BasicStatistics = tanks_delta.iter().map(|tank| tank.statistics.all).sum();
    let battle_life_time: i64 = tanks_delta
        .iter()
        .map(|tank| tank.statistics.battle_life_time.num_seconds())
        .sum();
    let current_win_rate = ConfidenceInterval::wilson_score_interval(
        current_info.statistics.n_battles(),
        current_info.statistics.n_wins(),
        Z::default(),
    );

    let navbar = html! {
        nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
            div.container {
                div.navbar-brand {
                    (home_button())

                    div.navbar-item title="Последний бой" {
                        time.(if current_info.has_recently_played() { "has-text-success-dark" } else if !current_info.is_active() { "has-text-danger-dark" } else { "" })
                            datetime=(current_info.base.last_battle_time.to_rfc3339())
                            title=(current_info.base.last_battle_time) {
                                (datetime(current_info.base.last_battle_time, Tense::Past))
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

            th.has-text-right {
                a data-sort="battle-life-time" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Время в боях" }
                    }
                }
            }

            th.has-text-right {
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

            th.has-text-right {
                a data-sort="win-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Процент побед" }
                    }
                }
            }

            th {
                a data-sort="true-win-rate-mean" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Истинный процент побед – это WR, скорректированный на число боев, CI 95%" { "TWR" } }
                    }
                }
            }

            th {
                a data-sort="frags-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Фраги за бой" }
                    }
                }
            }

            th {
                a data-sort="wins-per-hour" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Победы в час" }
                    }
                }
            }

            th {
                a data-sort="expected-wins-per-hour" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Число побед в час, скорректированное на число проведенных боев, CI 95%" { "TWPH" } }
                    }
                }
            }

            th {
                a data-sort="true-gold" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Доходность золотого бустера за бой, скорректированная на число проведенных боев, CI 95%" { "Ожидаемое золото" } }
                    }
                }
            }

            th.has-text-right {
                a data-sort="damage-dealt" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Общий урон" }
                    }
                }
            }

            th.has-text-right {
                a data-sort="damage-per-minute" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Урон в минуту" }
                    }
                }
            }

            th.has-text-right.is-white-space-nowrap {
                a data-sort="damage-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Урон за бой" }
                    }
                }
            }

            th.has-text-right {
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
                script type="module" defer {
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

                (headers())
                link rel="canonical" href=(uri!(get(account_id = account_id, period = _)));
                title { (current_info.nickname) " – Я – статист в World of Tanks Blitz!" }
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
                                                        p.title { (render_float(stats_delta.damage_per_battle(), 0)) }
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
                                                        p.title { (render_float(stats_delta.frags_per_battle(), 1)) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "В час" }
                                                        p.title { (render_float(stats_delta.frags as f64 / battle_life_time as f64 * 3600.0, 1)) }
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
                                                            span.has-text-grey-light { " ±" (render_float(100.0 * period_win_rate.margin, 1)) }
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
                                            p.card-header-title {
                                                span.icon-text {
                                                    span.icon.has-text-success { i.fas.fa-check {} }
                                                    span { "Победы" }
                                                }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "В час" }
                                                        p.title { (render_float(stats_delta.wins as f64 / battle_life_time as f64 * 3600.0, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-8-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text {
                                                    span.icon { i.fas.fa-heart.has-text-danger {} }
                                                    span { "Выживаемость" }
                                                }
                                            }
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
                                                            @let expected_period_survival_rate = ConfidenceInterval::wilson_score_interval(
                                                                stats_delta.battles, stats_delta.survived_battles, Z::default());
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
                                                (render_tank_tr(tank, &current_win_rate)?)
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
            }
        }
    };

    let result =
        Ok(CustomResponse::CachedMarkup("max-age=60, stale-while-revalidate=3600", markup));
    warn!(
        account_id = account_id,
        elapsed = %format_elapsed(&start_instant),
        "finished",
    );
    result
}

fn render_tank_tr(tank: &Tank, account_win_rate: &ConfidenceInterval) -> Result<Markup> {
    let markup = html! {
        @let vehicle = get_vehicle(tank.statistics.base.tank_id);
        @let true_win_rate = tank.statistics.all.true_win_rate();
        @let win_rate_ordering = true_win_rate.partial_cmp(account_win_rate);

        tr.(partial_cmp_class(win_rate_ordering)) {
            (vehicle_th(&vehicle))

            td.has-text-centered {
                @match vehicle.type_ {
                    TankType::Light => "ЛТ",
                    TankType::Medium => "СТ",
                    TankType::Heavy => "ТТ",
                    TankType::AT => "ПТ",
                    TankType::Unknown => "",
                }
            }

            td.has-text-right.is-white-space-nowrap data-sort="battle-life-time" data-value=(tank.statistics.battle_life_time.num_seconds()) {
                (format_duration(tank.statistics.battle_life_time.to_std()?))
            }

            td.has-text-right data-sort="battles" data-value=(tank.statistics.all.battles) {
                (tank.statistics.all.battles)
            }

            td data-sort="wins" data-value=(tank.statistics.all.wins) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (tank.statistics.all.wins) }
                }
            }

            @let win_rate = tank.statistics.all.current_win_rate();
            td.has-text-right data-sort="win-rate" data-value=(win_rate) {
                strong { (render_percentage(win_rate)) }
            }

            td.is-white-space-nowrap
                data-sort="true-win-rate-mean"
                data-value=(true_win_rate.mean)
            {
                span.icon-text.is-flex-wrap-nowrap {
                    span {
                        strong { span { (render_percentage(true_win_rate.mean)) } }
                        span.has-text-grey {
                            " ±" (render_float(100.0 * true_win_rate.margin, 1))
                        }
                    }
                    (partial_cmp_icon(win_rate_ordering))
                }
            }

            @let frags_per_battle = tank.statistics.all.frags_per_battle();
            td data-sort="frags-per-battle" data-value=(frags_per_battle) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-skull-crossbones.has-text-grey-light {} }
                    span { (render_float(frags_per_battle, 1)) }
                }
            }

            @let wins_per_hour = tank.wins_per_hour();
            td data-sort="wins-per-hour" data-value=(wins_per_hour) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (render_float(wins_per_hour, 1)) }
                }
            }

            @let expected_wins_per_hour = true_win_rate * tank.battles_per_hour();
            td.is-white-space-nowrap
                data-sort="expected-wins-per-hour"
                data-value=(expected_wins_per_hour.mean)
            {
                strong { (render_float(expected_wins_per_hour.mean, 1)) }
                span.has-text-grey {
                    (format!(" ±{:.1}", expected_wins_per_hour.margin))
                }
            }

            @let expected_gold = 10.0 + vehicle.tier as f64 * true_win_rate;
            td.is-white-space-nowrap data-sort="true-gold" data-value=(expected_gold.mean) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                    span {
                        strong { (render_float(expected_gold.mean, 1)) }
                        span.has-text-grey {
                            (format!(" ±{:.1}", expected_gold.margin))
                        }
                    }
                }
            }

            td.has-text-right data-sort="damage-dealt" data-value=(tank.statistics.all.damage_dealt) {
                (tank.statistics.all.damage_dealt)
            }

            @let damage_per_minute = tank.damage_per_minute();
            td.has-text-right data-sort="damage-per-minute" data-value=(damage_per_minute) {
                (format!("{:.0}", damage_per_minute))
            }

            @let damage_per_battle = tank.statistics.all.damage_dealt as f64 / tank.statistics.all.battles as f64;
            td.has-text-right data-sort="damage-per-battle" data-value=(damage_per_battle) {
                (format!("{:.0}", damage_per_battle))
            }

            td.has-text-right data-sort="survived-battles" data-value=(tank.statistics.all.survived_battles) {
                (tank.statistics.all.survived_battles)
            }

            @let survival_rate = tank.statistics.all.survived_battles as f64 / tank.statistics.all.battles as f64;
            td data-sort="survival-rate" data-value=(survival_rate) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-heart.has-text-danger {} }
                    span { (render_percentage(survival_rate)) }
                }
            }
        }
    };
    Ok(markup)
}
