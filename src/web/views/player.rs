use crate::api::wargaming::models::{AccountId, AccountInfo, Statistics, TankStatistics};
use crate::database;
use crate::database::Database;
use crate::logging::log_anyhow;
use crate::web::components::*;
use crate::web::responses::render_document;
use crate::web::State;
use chrono_humanize::HumanTime;
use maud::html;
use mongodb::options::InsertManyOptions;
use std::time::Instant;
use tide::{Response, StatusCode};

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
    let (_, tanks_stats) = state
        .api
        .get_tanks_stats(account_id)
        .await?
        .drain()
        .next()
        .unwrap();
    let win_percentage = account_info.statistics.all.win_percentage();

    let response = render_document(
        StatusCode::Ok,
        Some(account_info.nickname.as_str()),
        html! {
            nav.navbar.is-light role="navigation" aria-label="main navigation" {
                div.container {
                    div."navbar-brand" {
                        a."navbar-item" href="/" {
                            span.icon { i."fas"."fa-home" {} }
                            span { "Home" }
                        }
                        a.navbar-item href=(get_account_url(account_id)) {
                            span.icon { i.fas.fa-users {} }
                            span { "Player" }
                        }
                    }
                    div."navbar-menu" {
                        div.navbar-end {
                            form.navbar-item action="/" method="GET" {
                                (account_search("is-small", false))
                            }
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
                                    h2.subtitle title=(account_info.created_at) {
                                        "created " (HumanTime::from(account_info.created_at))
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
                                                p.title { (account_info.statistics.all.battles) }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Wins" }
                                                p.title {
                                                    span class=(win_percentage_class(win_percentage)) {
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
    );

    async_std::task::spawn(save_snapshots(
        state.database.clone(),
        account_info,
        tanks_stats,
    ));

    Ok(response)
}

pub fn get_account_url(account_id: AccountId) -> String {
    format!("/ru/{}", account_id)
}

impl Statistics {
    fn win_percentage(&self) -> f32 {
        100.0 * (self.wins as f32) / (self.battles as f32)
    }

    fn survival_percentage(&self) -> f32 {
        100.0 * (self.survived_battles as f32) / (self.battles as f32)
    }
}

/// Saves the account statistics to the database.
async fn save_snapshots(
    database: Database,
    account_info: AccountInfo,
    tanks_stats: Vec<TankStatistics>,
) {
    let start = Instant::now();
    log_anyhow(database::upsert(&database.accounts, &account_info).await);
    log_anyhow(database::upsert(&database.account_snapshots, &account_info).await);
    let _ = database
        // Unfortunately, I have to ignore errors here,
        // because the driver doesn't support the proper bulk operations.
        .tank_snapshots
        .insert_many(
            tanks_stats
                .iter()
                .map(Into::<crate::database::models::TankSnapshot>::into),
            InsertManyOptions::builder().ordered(false).build(),
        )
        .await;
    log::info!("Account snapshots saved in {:#?}.", Instant::now() - start);
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
