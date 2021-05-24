use crate::api::wargaming::AccountId;
use crate::web::{respond_with_document, respond_with_status, State};
use chrono_humanize::HumanTime;
use maud::html;
use tide::StatusCode;

pub fn get_user_url(account_id: AccountId) -> String {
    format!("/ru/{}", account_id)
}

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let account_id = match request.param("account_id") {
        Ok(account_id) => account_id,
        Err(_) => return respond_with_status(StatusCode::BadRequest),
    };
    let account_id: AccountId = match account_id.parse() {
        Ok(account_id) => account_id,
        Err(_) => return respond_with_status(StatusCode::BadRequest),
    };
    let state = request.state();
    let account_infos = state.api.get_account_info(account_id).await?;
    let account_info = match account_infos.values().next() {
        Some(account_info) => account_info,
        None => return respond_with_status(StatusCode::NotFound),
    };
    let statistics = &account_info.statistics.all;
    let win_rate = 100.0 * statistics.wins as f32 / statistics.battles as f32;
    let survival_rate = 100.0 * statistics.survived_battles as f32 / statistics.battles as f32;

    respond_with_document(
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
                    }
                }
            }

            section class="hero is-link is-small" {
                div class="hero-body" {
                    div class="container has-text-centered" {
                        p.title { (account_info.nickname) }
                    }
                }
            }

            section.section {
                div.container {
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
                                p.title { (format!("{:.2}", win_rate)) "%" }
                            }
                        }
                        div class="level-item has-text-centered" {
                            div {
                                p.heading { "Survival rate" }
                                p.title { (format!("{:.2}", survival_rate)) "%" }
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
        },
    )
}
