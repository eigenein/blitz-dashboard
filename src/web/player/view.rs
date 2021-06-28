use std::time::Duration as StdDuration;

use chrono_humanize::{Accuracy, HumanTime, Tense};
use clap::crate_name;
use maud::{html, Markup, Render, DOCTYPE};
use tide::StatusCode;

use crate::models::Vehicle;
use crate::statistics::ConfidenceInterval;
use crate::web::partials::footer::Footer;
use crate::web::partials::{account_search, headers, icon_text};
use crate::web::player::model::PlayerViewModel;
use crate::web::responses::html;
use crate::web::state::State;

const PERIOD_HOUR: StdDuration = StdDuration::from_secs(3600);
const PERIOD_2_HOURS: StdDuration = StdDuration::from_secs(2 * 3600);
const PERIOD_4_HOURS: StdDuration = StdDuration::from_secs(4 * 3600);
const PERIOD_8_HOURS: StdDuration = StdDuration::from_secs(8 * 3600);
const PERIOD_12_HOURS: StdDuration = StdDuration::from_secs(12 * 3600);
const PERIOD_DAY: StdDuration = StdDuration::from_secs(86400);
const PERIOD_48_HOURS: StdDuration = StdDuration::from_secs(2 * 86400);
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
                    title { (model.nickname) " – " (crate_name!()) }
                }
                body {
                    nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                        div.container {
                            div."navbar-brand" {
                                div.navbar-item {
                                    div.buttons {
                                        a.button.is-link href="/" {
                                            span.icon { i.fas.fa-home {} }
                                            span { "Home" }
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
                                                        p.heading { "Account age" }
                                                        p.title title=(model.created_at) {
                                                            (HumanTime::from(model.created_at).to_text_en(Accuracy::Rough, Tense::Present))
                                                        }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Tanks played" }
                                                        p.title { (model.total_tanks) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Battles" }
                                                        p.title { (model.total_battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Last battle" }
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

                            div.tabs.is-boxed {
                                ul {
                                    li.(if model.period == PERIOD_HOUR { "is-active" } else { "" }) {
                                        a href="?period=1h" { "Hour" }
                                    }
                                    li.(if model.period == PERIOD_2_HOURS { "is-active" } else { "" }) {
                                        a href="?period=2h" { "2 hours" }
                                    }
                                    li.(if model.period == PERIOD_4_HOURS { "is-active" } else { "" }) {
                                        a href="?period=4h" { "4 hours" }
                                    }
                                    li.(if model.period == PERIOD_8_HOURS { "is-active" } else { "" }) {
                                        a href="?period=8h" { "8 hours" }
                                    }
                                    li.(if model.period == PERIOD_12_HOURS { "is-active" } else { "" }) {
                                        a href="?period=12h" { "12 hours" }
                                    }
                                    li.(if model.period == PERIOD_DAY { "is-active" } else { "" }) {
                                        a href="?period=1d" { "24 hours" }
                                    }
                                    li.(if model.period == PERIOD_48_HOURS { "is-active" } else { "" }) {
                                        a href="?period=2d" { "48 hours" }
                                    }
                                    li.(if model.period == PERIOD_WEEK { "is-active" } else { "" }) {
                                        a href="?period=1w" { "Week" }
                                    }
                                    li.(if model.period == PERIOD_MONTH { "is-active" } else { "" }) {
                                        a href="?period=1M" { "Month" }
                                    }
                                    li.(if model.period == PERIOD_YEAR { "is-active" } else { "" }) {
                                        a href="?period=1y" { "Year" }
                                    }
                                }
                            }

                            @if model.warn_no_previous_account_info {
                                article.message.is-warning {
                                    div.message-body {
                                        "We haven't crawled this account at that moment in past. "
                                        strong { "Showing the all-time information." }
                                    }
                                }
                            }

                            div.tile.is-ancestor {
                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-sort-numeric-up-alt", "Battles")) }
                                        }
                                        div.card-content {
                                            div.level {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Total" }
                                                        p.title { (model.statistics.battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Wins" }
                                                        p.title { (model.statistics.wins) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Survived" }
                                                        p.title { (model.statistics.survived_battles) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.tile."is-4".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-house-damage", "Damage dealt")) }
                                        }
                                        div.card-content {
                                            div.level {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Total" }
                                                        p.title { (model.statistics.damage_dealt) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Mean" }
                                                        p.title title=(model.damage_dealt_mean) { (format!("{:.0}", model.damage_dealt_mean)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div.tile.is-ancestor {
                                @if let Some(wins) = &model.wins {
                                    div.tile."is-4".is-parent {
                                        div.tile.is-child.card {
                                            header.card-header {
                                                p.card-header-title { (icon_text("fas fa-percentage", "Win ratio")) }
                                            }
                                            div.card-content {
                                                (render_confidence_interval_level(wins))
                                            }
                                        }
                                    }
                                }

                                @if let Some(survival) = &model.survival {
                                    div.tile."is-4".is-parent {
                                        div.tile.is-child.card {
                                            header.card-header {
                                                p.card-header-title { (icon_text("fas fa-heart", "Survival ratio")) }
                                            }
                                            div.card-content {
                                                (render_confidence_interval_level(survival))
                                            }
                                        }
                                    }
                                }

                                @if let Some(hits) = &model.hits {
                                    div.tile."is-4".is-parent {
                                        div.tile.is-child.card {
                                            header.card-header {
                                                p.card-header-title { (icon_text("fas fa-bullseye", "Hit ratio")) }
                                            }
                                            div.card-content {
                                                (render_confidence_interval_level(hits))
                                            }
                                        }
                                    }
                                }
                            }

                            @if !model.tank_snapshots.is_empty() {
                                div.box {
                                    div.table-container {
                                        table.table.is-hoverable.is-striped.is-fullwidth {
                                            thead {
                                                tr {
                                                    th { (icon_text("fas fa-truck-monster", "Vehicle")) }
                                                    th { "Battles" }
                                                    th { "Wins" }
                                                    th { "Lower win ratio" }
                                                    th.has-text-info { "Win ratio" }
                                                    th { "Upper win ratio" }
                                                    th { "Survived" }
                                                    th { "Damage dealt" }
                                                    th { "Mean damage" }
                                                    th.has-text-warning { abbr title="Mean gold booster earnings" { "MGBE" } }
                                                }
                                            }
                                            tbody {
                                                @for snapshot in &model.tank_snapshots {
                                                    @let vehicle = state.get_vehicle(snapshot.tank_id).await?;
                                                    tr {
                                                        th { (vehicle.as_ref()) }
                                                        td { (snapshot.all_statistics.battles) }
                                                        td { (snapshot.all_statistics.wins) }
                                                        (render_confidence_interval_td(snapshot.all_statistics.battles, snapshot.all_statistics.wins))
                                                        td { (snapshot.all_statistics.survived_battles) }
                                                        td { (snapshot.all_statistics.damage_dealt) }
                                                        td { (format!("{:.0}", f64::from(snapshot.all_statistics.damage_dealt) / f64::from(snapshot.all_statistics.battles))) }
                                                        td { (format!("{:.1}", 10.0 + f64::from(vehicle.tier) * f64::from(snapshot.all_statistics.wins) / f64::from(snapshot.all_statistics.battles))) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            article.message.is-info {
                                div.message-body.content {
                                    ul."mt-0" {
                                        li {
                                            "Lower and upper bounds above refer to 90% "
                                            a href="https://en.wikipedia.org/wiki/Confidence_interval" { "confidence intervals" }
                                            "."
                                        }
                                        li {
                                            "Information is cached for a minute."
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

impl Render for &Vehicle {
    fn render(&self) -> Markup {
        let tier = html! {
            @match self.tier {
                1 => "Ⅰ ",
                2 => "Ⅱ ",
                3 => "Ⅲ ",
                4 => "Ⅳ ",
                5 => "Ⅴ ",
                6 => "Ⅵ ",
                7 => "Ⅶ ",
                8 => "Ⅷ ",
                9 => "Ⅸ ",
                10 => "Ⅹ ",
                _ => "",
            }
        };
        let class = if self.is_premium {
            "has-text-warning-dark"
        } else {
            ""
        };
        html! { span.(class) title=(self.tank_id) { (tier) (self.name) } }
    }
}

impl ConfidenceInterval {
    fn get_percentages(&self) -> (f64, f64, f64) {
        let mean = self.mean * 100.0;
        let margin = self.margin * 100.0;
        let lower = (mean - margin).max(0.0);
        let upper = (mean + margin).min(100.0);
        (lower, mean, upper)
    }
}

fn render_confidence_interval_level(interval: &ConfidenceInterval) -> Markup {
    let (lower, mean, upper) = interval.get_percentages();

    html! {
        div.level {
            div.level-item.has-text-centered {
                div {
                    p.heading { "Lower" }
                    p.title."is-5" title=(lower) { (format!("{:.1}%", lower)) }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Mean" }
                    p.title title=(mean) { (format!("{:.1}%", mean)) }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Upper" }
                    p.title."is-5" title=(upper) { (format!("{:.1}%", upper)) }
                }
            }
        }
    }
}

fn render_confidence_interval_td(n_trials: i32, n_successes: i32) -> Markup {
    let (lower, mean, upper) = match ConfidenceInterval::from_proportion_90(n_trials, n_successes) {
        Some(interval) => interval.get_percentages(),
        None => (0.0, 0.0, 0.0),
    };
    html! {
        td { (icon_text("fas fa-angle-down", &format!("{:.1}%", lower))) }
        td { strong.has-text-info { (format!("{:.1}%", mean)) } }
        td { (icon_text("fas fa-angle-up", &format!("{:.1}%", upper))) }
    }
}
