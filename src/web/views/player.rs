use crate::api::wargaming::models::AccountId;
use crate::web::components::*;
use crate::web::models::PlayerViewModel;
use crate::web::partials::{footer, header};
use crate::web::responses::render_document;
use crate::web::State;
use chrono_humanize::HumanTime;
use maud::html;
use tide::{Response, StatusCode};

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = match request.param("account_id")?.parse() {
        Ok(account_id) => PlayerViewModel::new(account_id, request.state()).await?,
        Err(_) => return Ok(Response::new(StatusCode::BadRequest)),
    };

    Ok(render_document(
        StatusCode::Ok,
        Some(model.nickname.as_str()),
        html! {
            (header(model.account_id))

            section.section {
                div.container {
                    div class="tile is-ancestor" {
                        div class="tile is-4 is-parent" {
                            div class="tile is-child card" {
                                header class="card-header" {
                                    p class="card-header-title" { (icon_text("fas fa-user", "Player")) }
                                }
                                div class="card-content" {
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
                                                p.title { (model.all_statistics.battles) }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Wins" }
                                                p.title {
                                                    span class=(win_percentage_class(model.wins)) {
                                                        (format!("{:.2}", model.wins)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Survival" }
                                                p.title {
                                                    (format!("{:.2}", model.survival)) "%"
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Last battle" }
                                                p.title title=(model.last_battle_time) {
                                                    (HumanTime::from(model.last_battle_time))
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
        },
    ))
}

pub fn get_account_url(account_id: AccountId) -> String {
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
