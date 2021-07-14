use std::time::Duration as StdDuration;

use chrono_humanize::Tense;
use humantime::format_duration;
use maud::{html, Markup, DOCTYPE};
use rocket::response::content::Html;

use crate::statistics::wilson_score_interval;
use crate::tankopedia::Tankopedia;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::web::helpers::{render_f64, render_nation, render_tier, render_vehicle_name};
use crate::web::partials::{account_search, datetime, footer, headers, icon_text};
use crate::web::state::State;

use super::models;
use super::models::ViewModel;

#[rocket::get("/ru/<account_id>?<sort>&<period>")]
pub async fn get(
    account_id: i32,
    sort: Option<String>,
    period: Option<String>,
    state: &rocket::State<State>,
    tankopedia: &rocket::State<Tankopedia>,
    account_info_cache: &rocket::State<AccountInfoCache>,
) -> crate::web::result::Result<Html<String>> {
    let model = ViewModel::new(
        account_id,
        sort,
        period,
        &state,
        &account_info_cache,
        &tankopedia,
    )
    .await?;

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                title { (model.nickname) " – Я статист!" }
            }
            body {
                (state.tracking_code)
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
                                    (account_search("", &model.nickname, false))
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
                                        p.card-header-title { (icon_text("fas fa-user", &model.nickname)) }
                                    }
                                    div.card-content {
                                        div.level {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Возраст" }
                                                    p.title title=(model.created_at) {
                                                        (datetime(model.created_at, Tense::Present))
                                                    }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Танков" }
                                                    p.title { (model.total_tanks) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Боев" }
                                                    p.title { (model.total_battles) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Последний бой" }
                                                    p.title.(if model.has_recently_played { "has-text-success" } else if !model.is_active { "has-text-danger" } else { "" }) {
                                                        time
                                                            datetime=(model.last_battle_time.to_rfc3339())
                                                            title=(model.last_battle_time) {
                                                                (datetime(model.last_battle_time, Tense::Past))
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
                                (render_period_li(&model, StdDuration::from_secs(3600), "Час")?)
                                (render_period_li(&model, StdDuration::from_secs(2 * 3600), "2 часа")?)
                                (render_period_li(&model, StdDuration::from_secs(4 * 3600), "4 часа")?)
                                (render_period_li(&model, StdDuration::from_secs(8 * 3600), "8 часов")?)
                                (render_period_li(&model, StdDuration::from_secs(12 * 3600), "12 часов")?)
                                (render_period_li(&model, StdDuration::from_secs(86400), "24 часа")?)
                                (render_period_li(&model, StdDuration::from_secs(2 * 86400), "2 дня")?)
                                (render_period_li(&model, StdDuration::from_secs(3 * 86400), "3 дня")?)
                                (render_period_li(&model, StdDuration::from_secs(7 * 86400), "Неделя")?)
                                (render_period_li(&model, StdDuration::from_secs(2630016), "Месяц")?)
                                (render_period_li(&model, StdDuration::from_secs(2 * 2630016), "2 месяца")?)
                                (render_period_li(&model, StdDuration::from_secs(3 * 2630016), "3 месяца")?)
                                (render_period_li(&model, StdDuration::from_secs(31557600), "Год")?)
                            }
                        }

                        @if model.warn_no_previous_account_info {
                            article.message.is-warning {
                                div.message-body {
                                    strong { "Отображается статистика за все время." }
                                    " У нас нет сведений об аккаунте за этот период."
                                }
                            }
                        }
                        @if model.last_battle_time >= model.before && model.statistics.battles == 0 {
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
                                                    p.title { (model.statistics.battles) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Победы" }
                                                    p.title { (model.statistics.wins) }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Выжил" }
                                                    p.title { (model.statistics.survived_battles) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            @if model.statistics.battles != 0 {
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
                                                        p.title { (model.statistics.damage_dealt) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_f64(model.statistics.damage_dealt as f64 / model.statistics.battles as f64, 0)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            @if model.statistics.battles != 0 {
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
                                                        p.title { (model.statistics.frags) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_f64(model.statistics.frags as f64 / model.statistics.battles as f64, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div.tile.is-ancestor {
                            @if model.statistics.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-percentage", "Победы")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(model.statistics.battles, model.statistics.wins))
                                        }
                                    }
                                }
                            }

                            @if model.statistics.battles != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-heart", "Выживаемость")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(model.statistics.battles, model.statistics.survived_battles))
                                        }
                                    }
                                }
                            }

                            @if model.statistics.shots != 0 {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-bullseye", "Попадания")) }
                                        }
                                        div.card-content {
                                            (render_confidence_interval_level(model.statistics.shots, model.statistics.hits))
                                        }
                                    }
                                }
                            }
                        }

                        @if !model.rows.is_empty() {
                            div.box {
                                div.table-container {
                                    table#vehicles.table.is-hoverable.is-striped.is-fullwidth {
                                        thead {
                                            tr {
                                                th { "Техника" }
                                                (render_vehicles_th(&model, models::SORT_BY_TIER, html! { "Уровень" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_NATION, html! { "Нация" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_VEHICLE_TYPE, html! { "Тип" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_BATTLES, html! { "Бои" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_WINS, html! { "Победы" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_WIN_RATE, html! { "Текущий процент побед" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_TRUE_WIN_RATE, html! { "Ожидаемый процент побед" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_GOLD, html! { abbr title="Текущий доход от золотых бустеров за бой, если они были установлены" { "Заработанное золото" } })?)
                                                (render_vehicles_th(&model, models::SORT_BY_TRUE_GOLD, html! { abbr title="Средняя ожидаемая доходность золотого бустера за бой" { "Ожидаемое золото" } })?)
                                                (render_vehicles_th(&model, models::SORT_BY_DAMAGE_DEALT, html! { "Ущерб" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_DAMAGE_PER_BATTLE, html! { "Ущерб за бой" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_SURVIVED_BATTLES, html! { "Выжил" })?)
                                                (render_vehicles_th(&model, models::SORT_BY_SURVIVAL_RATE, html! { "Выживаемость" })?)
                                            }
                                        }
                                        tbody {
                                            @for row in &model.rows {
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

fn render_period_li(
    model: &ViewModel,
    period: StdDuration,
    text: &'static str,
) -> crate::Result<Markup> {
    Ok(html! {
        li.(if model.period == period { "is-active" } else { "" }) {
            a href=(format!("?sort={}&period={}#period", model.sort, format_duration(period))) { (text) }
        }
    })
}

fn render_vehicles_th(model: &ViewModel, sort: &str, markup: Markup) -> crate::Result<Markup> {
    Ok(html! {
        th {
            a href=(format!("?sort={}&period={}#vehicles", sort, format_duration(model.period))) {
                span.icon-text.is-flex-wrap-nowrap {
                    @if model.sort == sort { span.icon { i.fas.fa-angle-down {} } }
                    span { (markup) }
                }
            }
        }
    })
}

fn render_confidence_interval_level(n_trials: i32, n_successes: i32) -> Markup {
    let mean = 100.0 * n_successes as f64 / n_trials as f64;
    let (p, margin) = wilson_score_interval(n_trials, n_successes);
    let lower = 100.0 * (p - margin);
    let upper = 100.0 * (p + margin);

    html! {
        div.level {
            div.level-item.has-text-centered {
                div {
                    p.heading { "Нижнее" }
                    p.title."is-5" { (render_f64(lower, 1)) "%" }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Среднее" }
                    p.title { (render_f64(mean, 1)) "%" }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Верхнее" }
                    p.title."is-5" { (render_f64(upper, 1)) "%" }
                }
            }
        }
    }
}

fn margin_class(value: f64, level_success: f64, level_warning: f64) -> &'static str {
    match value {
        _ if value < level_success => "has-text-success",
        _ if value < level_warning => "has-text-warning",
        _ => "has-text-danger",
    }
}
