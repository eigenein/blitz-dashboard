use crate::api::wargaming::models::{AccountId, AccountInfoStatisticsDetails};
use crate::database;
use crate::web::components::*;
use crate::web::responses::render_document;
use crate::web::State;
use chrono_humanize::HumanTime;
use maud::html;
use tide::{Response, StatusCode};

pub fn get_account_url(account_id: AccountId) -> String {
    format!("/ru/{}", account_id)
}

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let account_id: AccountId = match request.param("account_id")?.parse() {
        Ok(account_id) => account_id,
        Err(_) => return Ok(Response::new(StatusCode::BadRequest)),
    };
    let state = request.state();
    let mut account_infos = state.api.get_account_info(account_id).await?;
    let account_info = match account_infos.drain().next() {
        Some((_, account_info)) => account_info,
        None => return Ok(Response::new(StatusCode::NotFound)),
    };
    let win_percentage = account_info.statistics.all.win_percentage();

    // TODO: ignore errors here, only log them.
    database::upsert(&state.database.accounts, &account_info).await?;
    database::upsert(&state.database.account_snapshots, &account_info).await?;

    Ok(render_document(
        StatusCode::Ok,
        Some(account_info.nickname.as_str()),
        html! {
            nav.navbar."is-link" role="navigation" aria-label="main navigation" {
                div.container {
                    div."navbar-brand" {
                        a."navbar-item" href="/" {
                            span.icon { i."fas"."fa-home" {} }
                            span { "Home" }
                        }
                        a."navbar-item" href=(get_account_url(account_id)) {
                            span.icon { i."fas"."fa-user" {} }
                            span { "Player" }
                        }
                    }
                }
            }

            section.section {
                div.container {
                    div class="tile is-ancestor" {
                        div class="tile is-4 is-parent" {
                            div class="tile is-child card" {
                                header class="card-header" {
                                    p class="card-header-title" { (icon_text("fas fa-user", "Player")) }
                                }
                                div class="card-content" {
                                    h1.title { (account_info.nickname) }
                                    h2.subtitle title=(account_info.created_at) { "Created " (HumanTime::from(account_info.created_at)) }
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
                                                p.title { (account_info.statistics.all.battles) }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Wins" }
                                                p.title {
                                                    span class=(win_rate_class(win_percentage)) {
                                                        (format!("{:.2}", win_percentage)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Survival" }
                                                p.title {
                                                    (format!("{:.2}", account_info.statistics.all.survival_percentage())) "%"
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Last battle" }
                                                p.title title=(account_info.last_battle_time) {
                                                    (HumanTime::from(account_info.last_battle_time))
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
        },
    ))
}

impl AccountInfoStatisticsDetails {
    pub fn win_percentage(&self) -> f32 {
        100.0 * (self.wins as f32) / (self.battles as f32)
    }

    pub fn survival_percentage(&self) -> f32 {
        100.0 * (self.survived_battles as f32) / (self.battles as f32)
    }
}

fn win_rate_class(percentage: f32) -> &'static str {
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
