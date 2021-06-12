use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup, Render};
use tide::StatusCode;

use crate::models::Vehicle;
use crate::web::components::footer::Footer;
use crate::web::components::icon_text;
use crate::web::partials::header;
use crate::web::player::model::PlayerViewModel;
use crate::web::responses::render_document;
use crate::web::state::State;

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = PlayerViewModel::new(&request).await?;
    let footer = Footer::new(&request.state()).await?;
    Ok(render_document(
        StatusCode::Ok,
        Some(model.nickname.as_str()),
        html! {
            (header(model.account_id))

            section.section {
                div.container {
                    div.tile.is-ancestor {
                        div.tile."is-4".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-user", "Player")) }
                                }
                                div.card-content {
                                    h1.title { (model.nickname) }
                                    h2.subtitle title=(model.created_at) {
                                        "created " (HumanTime::from(model.created_at))
                                    }
                                }
                            }
                        }

                        div class="tile is-8 is-parent" {
                            div class="tile is-child card" {
                                header class="card-header" {
                                    p class="card-header-title" { (icon_text("fas fa-table", "Overview")) }
                                }
                                div class="card-content" {
                                    div.level {
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Battles" }
                                                p.title { (model.n_battles) }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Wins" }
                                                p.title {
                                                    span class=(win_percentage_class(model.wins)) {
                                                        (format!("{:.1}", model.wins)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Survival" }
                                                p.title {
                                                    (format!("{:.1}", model.survival)) "%"
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Hits" }
                                                p.title {
                                                    span {
                                                        (format!("{:.1}", model.hits)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
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

                    div.tile.is-ancestor {
                        div.tile."is-6".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-truck-monster", "Vehicles")) }
                                }
                                div.card-content {
                                    table.table.is-striped.is-hoverable.is-fullwidth {
                                        tbody {
                                            @if let Some((vehicle, duration)) = model.longest_life_time_vehicle {
                                                tr {
                                                    td { "Most lived vehicle" }
                                                    td { (vehicle) }
                                                    td title=(duration) { (HumanTime::from(duration).to_text_en(Accuracy::Rough, Tense::Present)) }
                                                }
                                            }
                                            @if let Some((vehicle, n_battles)) = model.most_played_vehicle {
                                                tr {
                                                    td { "Most played vehicle" }
                                                    td { (vehicle) }
                                                    td { (n_battles) " battles" }
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

            (footer)
        },
    ))
}

pub fn get_account_url(account_id: i32) -> String {
    format!("/ru/{}", account_id)
}

fn win_percentage_class(percentage: f32) -> &'static str {
    if percentage < 45.0 {
        "has-text-danger"
    } else if percentage < 50.0 {
        "has-text-warning"
    } else if percentage < 60.0 {
        "has-text-primary"
    } else {
        "has-text-success"
    }
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
