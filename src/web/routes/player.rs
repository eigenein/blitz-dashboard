use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use chrono_humanize::Tense;
use clap::crate_version;
use humantime::{format_duration, parse_duration};
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
use crate::statistics::ConfidenceInterval;
use crate::tankopedia::get_vehicle;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::cache::account::tanks::AccountTanksCache;
use crate::web::partials::{account_search, datetime, footer, headers, icon_text};
use crate::web::TrackingCode;

pub mod partials;

#[rocket::get("/ru/<account_id>?<period>")]
pub async fn get(
    account_id: i32,
    period: Option<String>,
    database: &State<PgPool>,
    account_info_cache: &State<AccountInfoCache>,
    tracking_code: &State<TrackingCode>,
    account_tanks_cache: &State<AccountTanksCache>,
) -> crate::web::result::Result<Html<String>> {
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

    let stats_delta = match &previous_info {
        Some(previous_info) => &current_info.statistics.all - &previous_info.statistics.all,
        None => current_info.statistics.all,
    };
    let total_win_rate = ConfidenceInterval::default_wilson_score_interval(
        current_info.statistics.all.battles,
        current_info.statistics.all.wins,
    );

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                title { (current_info.general.nickname) " – Я статист!" }
                script defer="true" src=(concat!("/static/player.js?", crate_version!())) {};
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
                                (render_period_li(period, StdDuration::from_secs(3600), "Час"))
                                (render_period_li(period, StdDuration::from_secs(2 * 3600), "2 часа"))
                                (render_period_li(period, StdDuration::from_secs(4 * 3600), "4 часа"))
                                (render_period_li(period, StdDuration::from_secs(8 * 3600), "8 часов"))
                                (render_period_li(period, StdDuration::from_secs(12 * 3600), "12 часов"))
                                (render_period_li(period, StdDuration::from_secs(86400), "24 часа"))
                                (render_period_li(period, StdDuration::from_secs(2 * 86400), "2 дня"))
                                (render_period_li(period, StdDuration::from_secs(3 * 86400), "3 дня"))
                                (render_period_li(period, StdDuration::from_secs(7 * 86400), "Неделя"))
                                (render_period_li(period, StdDuration::from_secs(2630016), "Месяц"))
                                (render_period_li(period, StdDuration::from_secs(2 * 2630016), "2 месяца"))
                                (render_period_li(period, StdDuration::from_secs(3 * 2630016), "3 месяца"))
                                (render_period_li(period, StdDuration::from_secs(31557600), "Год"))
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

                        @if current_info.general.last_battle_time >= before && stats_delta.battles == 0 {
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
                                                        p.title { (render_f64(stats_delta.damage_dealt as f64 / stats_delta.battles as f64, 0)) }
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
                                                        p.title { (render_f64(stats_delta.frags as f64 / stats_delta.battles as f64, 1)) }
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
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-percentage", "Победы")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(stats_delta.battles, stats_delta.wins))
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
                                            (render_confidence_interval_level(stats_delta.battles, stats_delta.survived_battles))
                                        }
                                    }
                                }
                            }

                            @if stats_delta.shots != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-bullseye", "Попадания")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(stats_delta.shots, stats_delta.hits))
                                        }
                                    }
                                }
                            }
                        }

                        @if !tanks_delta.is_empty() {
                            div.box {
                                div.table-container {
                                    table#vehicles.table.is-hoverable.is-striped.is-fullwidth {
                                        thead {
                                            @let period = format_duration(period);
                                            tr {
                                                th { "Техника" }
                                                th#by-tier {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-tier", period)) { "Уровень" } }
                                                    }
                                                }
                                                th { "Тип" }
                                                th#by-battles {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-battles", period)) { "Бои" } }
                                                    }
                                                }
                                                th#by-wins {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-wins", period)) { "Победы" } }
                                                    }
                                                }
                                                th#by-win-rate {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-win-rate", period)) { "Текущий процент побед" } }
                                                    }
                                                }
                                                th#by-true-win-rate {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-true-win-rate", period)) { "Ожидаемый процент побед" } }
                                                    }
                                                }
                                                th#by-gold {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-gold", period)) { abbr title="Текущий доход от золотых бустеров за бой, если они были установлены" { "Заработанное золото" } } }
                                                    }
                                                }
                                                th#by-true-gold {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-true-gold", period)) { abbr title="Средняя ожидаемая доходность золотого бустера за бой" { "Ожидаемое золото" } } }
                                                    }
                                                }
                                                th#by-damage-dealt {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-damage-dealt", period)) { "Ущерб" } }
                                                    }
                                                }
                                                th#by-damage-per-battle {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-damage-per-battle", period)) { "Ущерб за бой" } }
                                                    }
                                                }
                                                th#by-survived-battles {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-survived-battles", period)) { "Выжил" } }
                                                    }
                                                }
                                                th#by-survival-rate {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { a href=(format!("?period={}#by-survival-rate", period)) { "Выживаемость" } }
                                                    }
                                                }
                                            }
                                        }
                                        tbody {
                                            @for tank in &tanks_delta {
                                                @let vehicle = get_vehicle(tank.tank_id);
                                                @let win_rate = tank.all_statistics.wins as f64 / tank.all_statistics.battles as f64;
                                                @let expected_win_rate = ConfidenceInterval::default_wilson_score_interval(tank.all_statistics.battles, tank.all_statistics.wins);
                                                tr {
                                                    th.is-white-space-nowrap {
                                                        (render_nation(&vehicle.nation))
                                                        " " (render_vehicle_name(&vehicle))
                                                    }
                                                    td.has-text-centered data-sort="#by-tier" data-value=(vehicle.tier) {
                                                        @if let Some(tier_markup) = TIER_MARKUP.get(&vehicle.tier) {
                                                            strong { (tier_markup) }
                                                        }
                                                    }
                                                    td {
                                                        (format!("{:?}", vehicle.type_))
                                                    }
                                                    td data-sort="#by-battles" data-value=(tank.all_statistics.battles) {
                                                        (tank.all_statistics.battles)
                                                    }
                                                    td data-sort="#by-wins" data-value=(tank.all_statistics.wins) {
                                                        (tank.all_statistics.wins)
                                                    }
                                                    td.has-text-info data-sort="#by-win-rate" data-value=(win_rate) {
                                                        strong { (render_f64(100.0 * win_rate, 1)) "%" }
                                                    }
                                                    @let win_rate_ordering = expected_win_rate.partial_cmp(&total_win_rate);
                                                    td.has-text-centered.(partial_cmp_class(win_rate_ordering))
                                                        data-sort="#by-true-win-rate"
                                                        data-value=(expected_win_rate.mean)
                                                    {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span {
                                                                (partial_cmp_icon(win_rate_ordering))
                                                                strong { (render_f64(100.0 * expected_win_rate.mean, 1)) "%" }
                                                                span.(margin_class(expected_win_rate.margin, 0.1, 0.25)) {
                                                                    " ±" (render_f64(expected_win_rate.margin * 100.0, 1))
                                                                }
                                                            }
                                                        }
                                                    }
                                                    @let gold = 10.0 + vehicle.tier as f64 * win_rate;
                                                    td data-sort="#by-gold" data-value=(gold) {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                                                            span { strong { (render_f64(gold, 1)) } }
                                                        }
                                                    }
                                                    @let expected_gold = 10.0 + vehicle.tier as f64 * expected_win_rate;
                                                    td.is-white-space-nowrap data-sort="#by-true-gold" data-value=(expected_gold.mean) {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                                                            span {
                                                                strong { (render_f64(expected_gold.mean, 1)) }
                                                                span.(margin_class(expected_gold.margin, 2.0, 3.0)) {
                                                                    " ±" (render_f64(expected_gold.margin, 1))
                                                                }
                                                            }
                                                        }
                                                    }
                                                    td data-sort="#by-damage-dealt" data-value=(tank.all_statistics.damage_dealt) {
                                                        (tank.all_statistics.damage_dealt)
                                                    }
                                                    @let damage_per_battle = tank.all_statistics.damage_dealt as f64 / tank.all_statistics.battles as f64;
                                                    td data-sort="#by-damage-per-battle" data-value=(damage_per_battle) {
                                                        (render_f64(damage_per_battle, 0))
                                                    }
                                                    td data-sort="#by-survived-battles" data-value=(tank.all_statistics.survived_battles) {
                                                        (tank.all_statistics.survived_battles)
                                                    }
                                                    @let survival_rate = tank.all_statistics.survived_battles as f64 / tank.all_statistics.battles as f64;
                                                    td data-sort="#by-survival-rate" data-value=(survival_rate) {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span.icon { i.fas.fa-heart.has-text-danger {} }
                                                            span { (render_f64(100.0 * survival_rate, 0)) "%" }
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
