use crate::api::wargaming::{AccountId, AccountInfoStatisticsDetails};
use crate::web::{respond_with_document, respond_with_status, State};
use chrono_humanize::HumanTime;
use maud::html;
use tide::StatusCode;

pub fn get_account_url(account_id: AccountId) -> String {
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
                        a."navbar-item" href=(get_account_url(account_id)) {
                            span.icon { i."fas"."fa-user" {} }
                            span { "Player" }
                        }
                    }
                }
            }

            section.section {
                div.container {}
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
                                p.title { (format!("{:.2}", account_info.statistics.all.win_percent())) "%" }
                            }
                        }
                        div class="level-item has-text-centered" {
                            div {
                                p.heading { "Survival rate" }
                                p.title { (format!("{:.2}", account_info.statistics.all.survival_percent())) "%" }
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

impl AccountInfoStatisticsDetails {
    pub fn win_percent(&self) -> f32 {
        100.0 * (self.wins as f32) / (self.battles as f32)
    }

    pub fn survival_percent(&self) -> f32 {
        100.0 * (self.survived_battles as f32) / (self.battles as f32)
    }
}
