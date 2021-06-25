use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup, Render};
use tide::StatusCode;

use crate::models::Vehicle;
use crate::statistics::ConfidenceInterval;
use crate::web::components::footer::Footer;
use crate::web::components::icon_text;
use crate::web::partials::header;
use crate::web::player::model::{PlayerViewModel, Since};
use crate::web::responses::render_document;
use crate::web::state::State;

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = PlayerViewModel::new(&request).await?;
    let account_url = get_account_url(model.account_id);
    let footer = Footer::new(&request.state()).await?;

    Ok(render_document(
        StatusCode::Ok,
        Some(model.nickname.as_str()),
        html! {
            (header(model.nickname.as_str()))

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
                                                p.title.(if model.has_recently_played { "has-text-success" } else if model.is_inactive { "has-text-danger" } else { "" })
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
                            li.(if model.since == Since::Hour { "is-active" } else { "" }) {
                                a href=(format!("{}?since=1h", account_url)) { "Hour" }
                            }
                            li.(if model.since == Since::FourHours { "is-active" } else { "" }) {
                                a href=(format!("{}?since=4h", account_url)) { "4 hours" }
                            }
                            li.(if model.since == Since::EightHours { "is-active" } else { "" }) {
                                a href=(format!("{}?since=8h", account_url)) { "8 hours" }
                            }
                            li.(if model.since == Since::TwelveHours { "is-active" } else { "" }) {
                                a href=(format!("{}?since=12h", account_url)) { "12 hours" }
                            }
                            li.(if model.since == Since::Day { "is-active" } else { "" }) {
                                a href=(format!("{}?since=1d", account_url)) { "24 hours" }
                            }
                            li.(if model.since == Since::Week { "is-active" } else { "" }) {
                                a href=(format!("{}?since=1w", account_url)) { "Week" }
                            }
                            li.(if model.since == Since::Month { "is-active" } else { "" }) {
                                a href=(format!("{}?since=1m", account_url)) { "Month" }
                            }
                            li.(if model.since == Since::Year { "is-active" } else { "" }) {
                                a href=(format!("{}?since=1y", account_url)) { "Year" }
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
                                                p.title { (model.period_battles) }
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
                                                p.title { (model.period_damage_dealt_total) }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Mean" }
                                                p.title title=(model.period_damage_dealt_mean) { (format!("{:.0}", model.period_damage_dealt_mean)) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div.tile.is-ancestor {
                        @if let Some(period_wins) = &model.period_wins {
                            div.tile."is-4".is-parent {
                                div.tile.is-child.card {
                                    header.card-header {
                                        p.card-header-title { (icon_text("fas fa-percentage", "Wins")) }
                                    }
                                    div.card-content {
                                        (period_wins)
                                    }
                                }
                            }
                        }

                        @if let Some(period_survival) = &model.period_survival {
                            div.tile."is-4".is-parent {
                                div.tile.is-child.card {
                                    header.card-header {
                                        p.card-header-title { (icon_text("fas fa-heart", "Survival")) }
                                    }
                                    div.card-content {
                                        (period_survival)
                                    }
                                }
                            }
                        }

                        @if let Some(period_hits) = &model.period_hits {
                            div.tile."is-4".is-parent {
                                div.tile.is-child.card {
                                    header.card-header {
                                        p.card-header-title { (icon_text("fas fa-bullseye", "Hits")) }
                                    }
                                    div.card-content {
                                        (period_hits)
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
        },
    ))
}

pub fn get_account_url(account_id: i32) -> String {
    format!("/ru/{}", account_id)
}

impl Render for Vehicle {
    fn render(&self) -> Markup {
        let tier = match self.tier {
            1 => "Ⅰ",
            2 => "Ⅱ",
            3 => "Ⅲ",
            4 => "Ⅳ",
            5 => "Ⅴ",
            6 => "Ⅵ",
            7 => "Ⅶ",
            8 => "Ⅷ",
            9 => "Ⅸ",
            10 => "Ⅹ",
            _ => "?",
        };
        html! {
            strong.(if self.is_premium { "has-text-warning-dark" } else { "" }) title=(self.tank_id) {
                (tier) " " (self.name)
            }
        }
    }
}

impl Render for &ConfidenceInterval {
    fn render(&self) -> Markup {
        let mean = self.mean * 100.0;
        let margin = self.margin * 100.0;
        let lower = (mean - margin).max(0.0);
        let upper = (mean + margin).min(100.0);

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
}
