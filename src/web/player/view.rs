use chrono_humanize::{Accuracy, HumanTime, Tense};
use clap::crate_name;
use maud::{html, Markup, PreEscaped, Render, DOCTYPE};
use tide::StatusCode;

use crate::models::Vehicle;
use crate::statistics::ConfidenceInterval;
use crate::web::partials::footer::Footer;
use crate::web::partials::{account_search, headers, icon_text};
use crate::web::player::model::{Period, PlayerViewModel};
use crate::web::responses::html;
use crate::web::state::State;

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
                                    li.(if model.period == Period::Hour { "is-active" } else { "" }) {
                                        a href="?period=1h" { "Hour" }
                                    }
                                    li.(if model.period == Period::FourHours { "is-active" } else { "" }) {
                                        a href="?period=4h" { "4 hours" }
                                    }
                                    li.(if model.period == Period::EightHours { "is-active" } else { "" }) {
                                        a href="?period=8h" { "8 hours" }
                                    }
                                    li.(if model.period == Period::TwelveHours { "is-active" } else { "" }) {
                                        a href="?period=12h" { "12 hours" }
                                    }
                                    li.(if model.period == Period::Day { "is-active" } else { "" }) {
                                        a href="?period=1d" { "24 hours" }
                                    }
                                    li.(if model.period == Period::Week { "is-active" } else { "" }) {
                                        a href="?period=1w" { "Week" }
                                    }
                                    li.(if model.period == Period::Month { "is-active" } else { "" }) {
                                        a href="?period=1m" { "Month" }
                                    }
                                    li.(if model.period == Period::Year { "is-active" } else { "" }) {
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
                                div.tile."is-2".is-parent {
                                    div.tile.is-child.card {
                                        header.card-header {
                                            p.card-header-title { (icon_text("fas fa-sort-numeric-up-alt", "Battles")) }
                                        }
                                        div.card-content {
                                            div.level {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Battles" }
                                                        p.title { (model.statistics.battles) }
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
                                                p.card-header-title { (icon_text("fas fa-percentage", "Wins")) }
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
                                                p.card-header-title { (icon_text("fas fa-heart", "Survival")) }
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
                                                p.card-header-title { (icon_text("fas fa-bullseye", "Hits")) }
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
                                                    th { "Lower wins" }
                                                    th { "Wins" }
                                                    th { "Upper wins" }
                                                    th { "Damage dealt" }
                                                    th { "Mean damage" }
                                                }
                                            }
                                            tbody {
                                                @for snapshot in &model.tank_snapshots {
                                                    tr {
                                                        th { (state.get_vehicle(snapshot.tank_id).await?.as_ref()) }
                                                        td { (snapshot.all_statistics.battles) }
                                                        (render_confidence_interval_td(snapshot.all_statistics.battles, snapshot.all_statistics.wins))
                                                        td { (snapshot.all_statistics.damage_dealt) }
                                                        td { (format!("{:.0}", f64::from(snapshot.all_statistics.damage_dealt) / f64::from(snapshot.all_statistics.battles))) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            article.message.is-info {
                                div.message-body {
                                    "Lower and upper bounds above refer to 90% "
                                    a href="https://en.wikipedia.org/wiki/Confidence_interval" { "confidence intervals" }
                                    "."
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
        let tier = PreEscaped(match self.tier {
            1 => "Ⅰ&nbsp;",
            2 => "Ⅱ&nbsp;",
            3 => "Ⅲ&nbsp;",
            4 => "Ⅳ&nbsp;",
            5 => "Ⅴ&nbsp;",
            6 => "Ⅵ&nbsp;",
            7 => "Ⅶ&nbsp;",
            8 => "Ⅷ&nbsp;",
            9 => "Ⅸ&nbsp;",
            10 => "Ⅹ&nbsp;",
            _ => "",
        });
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
        td { strong { (icon_text("fas fa-grip-lines", &format!("{:.1}%", mean))) } }
        td { (icon_text("fas fa-angle-up", &format!("{:.1}%", upper)))  }
    }
}
