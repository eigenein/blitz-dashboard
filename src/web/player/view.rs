use std::time::Duration as StdDuration;

use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup, DOCTYPE};
use tide::StatusCode;

use crate::statistics::wilson_score_interval_90;
use crate::web::helpers::{render_f64, render_nation, render_tier, render_vehicle_name};
use crate::web::partials::footer::Footer;
use crate::web::partials::{account_search, headers, icon_text};
use crate::web::player::models::PlayerViewModel;
use crate::web::responses::html;
use crate::web::state::State;

const PERIOD_HOUR: StdDuration = StdDuration::from_secs(3600);
const PERIOD_2_HOURS: StdDuration = StdDuration::from_secs(2 * 3600);
const PERIOD_4_HOURS: StdDuration = StdDuration::from_secs(4 * 3600);
const PERIOD_8_HOURS: StdDuration = StdDuration::from_secs(8 * 3600);
const PERIOD_12_HOURS: StdDuration = StdDuration::from_secs(12 * 3600);
const PERIOD_DAY: StdDuration = StdDuration::from_secs(86400);
const PERIOD_2_DAYS: StdDuration = StdDuration::from_secs(2 * 86400);
const PERIOD_3_DAYS: StdDuration = StdDuration::from_secs(3 * 86400);
const PERIOD_WEEK: StdDuration = StdDuration::from_secs(7 * 86400);
const PERIOD_MONTH: StdDuration = StdDuration::from_secs(2630016);
const PERIOD_YEAR: StdDuration = StdDuration::from_secs(31557600);

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = PlayerViewModel::new(&request).await?;
    let state = request.state();
    let footer = Footer::new(state).await?;

    Ok(html(
        StatusCode::Ok,
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    (headers())
                    title { (model.nickname) " – Я статист!" }
                }
                body {
                    nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                        div.container {
                            div."navbar-brand" {
                                div.navbar-item {
                                    div.buttons {
                                        a.button.is-link href="/" {
                                            span.icon { i.fas.fa-home {} }
                                            span { "На главную" }
                                        }
                                    }
                                }
                            }
                            div."navbar-menu" {
                                div.navbar-end {
                                    form.navbar-item action="/" method="GET" {
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
                                                            (HumanTime::from(model.created_at).to_text_en(Accuracy::Rough, Tense::Present))
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
                                                        p.title.(if model.has_recently_played { "has-text-success" } else if !model.is_active { "has-text-danger" } else { "" })
                                                            title=(model.last_battle_time) {
                                                            (HumanTime::from(model.last_battle_time))
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
                                    li.(if model.period == PERIOD_HOUR { "is-active" } else { "" }) {
                                        a href="?period=1h#period" { "Час" }
                                    }
                                    li.(if model.period == PERIOD_2_HOURS { "is-active" } else { "" }) {
                                        a href="?period=2h#period" { "2 часа" }
                                    }
                                    li.(if model.period == PERIOD_4_HOURS { "is-active" } else { "" }) {
                                        a href="?period=4h#period" { "4 часа" }
                                    }
                                    li.(if model.period == PERIOD_8_HOURS { "is-active" } else { "" }) {
                                        a href="?period=8h#period" { "8 часов" }
                                    }
                                    li.(if model.period == PERIOD_12_HOURS { "is-active" } else { "" }) {
                                        a href="?period=12h#period" { "12 часов" }
                                    }
                                    li.(if model.period == PERIOD_DAY { "is-active" } else { "" }) {
                                        a href="?period=1d#period" { "24 часа" }
                                    }
                                    li.(if model.period == PERIOD_2_DAYS { "is-active" } else { "" }) {
                                        a href="?period=2d#period" { "2 дня" }
                                    }
                                    li.(if model.period == PERIOD_3_DAYS { "is-active" } else { "" }) {
                                        a href="?period=3d#period" { "3 дня" }
                                    }
                                    li.(if model.period == PERIOD_WEEK { "is-active" } else { "" }) {
                                        a href="?period=1w#period" { "Неделя" }
                                    }
                                    li.(if model.period == PERIOD_MONTH { "is-active" } else { "" }) {
                                        a href="?period=1M#period" { "Месяц" }
                                    }
                                    li.(if model.period == PERIOD_YEAR { "is-active" } else { "" }) {
                                        a href="?period=1y#period" { "Год" }
                                    }
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

                            @if !model.tank_snapshots.is_empty() {
                                div.box {
                                    div.table-container {
                                        table#vehicles.table.is-hoverable.is-striped.is-fullwidth {
                                            thead {
                                                tr {
                                                    th { a href="#vehicles" { "Техника" } }
                                                    th.has-text-centered { a href="#vehicles" { "Уровень" } }
                                                    th.has-text-centered { a href="#vehicles" { "Нация" } }
                                                    th.has-text-centered { a href="#vehicles" { "Тип" } }
                                                    th { a href="#vehicles" { "Бои" } }
                                                    th { a href="#vehicles" { "Победы" } }
                                                    th { a href="#vehicles" { "Текущий процент побед" } }
                                                    th { a href="#vehicles" { "Ожидаемый процент побед" } }
                                                    th { a href="#vehicles" { abbr title="Текущий доход от золотых бустеров за бой, если они были установлены" { "Заработанное золото" } }}
                                                    th { a href="#vehicles" { abbr title="Средняя ожидаемая доходность золотого бустера за бой" { "Ожидаемое золото" } } }
                                                    th { a href="#vehicles" { "Ущерб" } }
                                                    th { a href="#vehicles" { "Ущерб за бой" } }
                                                    th { a href="#vehicles" { "Выжил" } }
                                                    th { a href="#vehicles" { "Выживаемость" } }
                                                    th { a href="#vehicles" { "Техника" } }
                                                }
                                            }
                                            tbody {
                                                @for snapshot in &model.tank_snapshots {
                                                    @let vehicle = state.get_vehicle(snapshot.tank_id).await?;
                                                    @let statistics = &snapshot.all_statistics;
                                                    @let win_rate = statistics.wins as f64 / statistics.battles as f64;
                                                    @let (estimated_win_rate, win_rate_margin) = wilson_score_interval_90(snapshot.all_statistics.battles, snapshot.all_statistics.wins);
                                                    @let survival_percentage = 100.0 * (statistics.survived_battles as f64) / (statistics.battles as f64);
                                                    @let mean_damage_dealt = statistics.damage_dealt as f64 / statistics.battles as f64;

                                                    tr {
                                                        th.is-white-space-nowrap { (render_vehicle_name(&vehicle)) }
                                                        td.has-text-centered { strong { (render_tier(vehicle.tier)) } }
                                                        td.has-text-centered { (render_nation(&vehicle.nation)) }
                                                        td { (format!("{:?}", vehicle.type_)) }
                                                        td { (snapshot.all_statistics.battles) }
                                                        td { (snapshot.all_statistics.wins) }
                                                        td.has-text-info { strong { (render_f64(100.0 * win_rate, 1)) "%" } }
                                                        td.has-text-centered.is-white-space-nowrap {
                                                            strong { (render_f64(100.0 * estimated_win_rate, 1)) "%" }
                                                            " ±"
                                                            (render_f64(win_rate_margin * 100.0, 1))
                                                        }
                                                        td {
                                                            span.icon-text.is-flex-wrap-nowrap {
                                                                span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                                                                span { strong { (render_f64(10.0 + vehicle.tier as f64 * win_rate, 1)) } }
                                                            }
                                                        }
                                                        td.is-white-space-nowrap {
                                                            span.icon-text.is-flex-wrap-nowrap {
                                                                span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                                                                span {
                                                                    strong { (render_f64(10.0 + vehicle.tier as f64 * estimated_win_rate, 1)) }
                                                                    " ±"
                                                                    (render_f64(vehicle.tier as f64 * win_rate_margin, 1))
                                                                }
                                                            }
                                                        }
                                                        td { (snapshot.all_statistics.damage_dealt) }
                                                        td { (render_f64(mean_damage_dealt, 0)) }
                                                        td { (snapshot.all_statistics.survived_battles) }
                                                        td {
                                                            span.icon-text.is-flex-wrap-nowrap {
                                                                span.icon { i.fas.fa-heart.has-text-danger {} }
                                                                span { (render_f64(survival_percentage, 0)) "%" }
                                                            }
                                                        }
                                                        th.is-white-space-nowrap { (render_vehicle_name(&vehicle)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    (footer)
                }
            }
        },
    ))
}

pub fn get_account_url(account_id: i32) -> String {
    format!("/ru/{}", account_id)
}

fn render_confidence_interval_level(n_trials: i32, n_successes: i32) -> Markup {
    let mean = 100.0 * n_successes as f64 / n_trials as f64;
    let (p, margin) = wilson_score_interval_90(n_trials, n_successes);
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
