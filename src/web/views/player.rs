use crate::api::wargaming::{AccountId, AccountInfo, AccountInfoStatisticsDetails};
use crate::web::components::*;
use crate::web::responses::document_response;
use crate::web::State;
use chrono_humanize::HumanTime;
use maud::html;
use mongodb::bson::doc;
use mongodb::options::ReplaceOptions;
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
    let account_infos = state.api.get_account_info(account_id).await?;
    let account_info = match account_infos.values().next() {
        Some(account_info) => account_info,
        None => return Ok(Response::new(StatusCode::NotFound)),
    };
    state
        .database
        .collection::<AccountInfo>("accounts")
        .replace_one(
            doc! { "account_id": account_info.id, "updated_at": account_info.updated_at.timestamp() },
            account_info,
            Some(ReplaceOptions::builder().upsert(true).build()),
        )
        .await?;
    let win_percentage = account_info.statistics.all.win_percentage();

    document_response(
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
                                    h2.subtitle { "Created " (HumanTime::from(account_info.created_at)) }
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
                                                p.heading { "Win rate" }
                                                p.title {
                                                    span class=(win_rate_class(win_percentage)) {
                                                        (format!("{:.2}", win_percentage)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Survival rate" }
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
    )
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
