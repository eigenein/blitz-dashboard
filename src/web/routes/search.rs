use chrono_humanize::Tense;
use maud::{html, DOCTYPE};
use rocket::response::content::Html;
use rocket::response::Redirect;
use rocket::{Responder, State};

use crate::logging::clear_user;
use crate::models::AccountInfo;
use crate::wargaming::WargamingApi;
use crate::web::partials::{account_search, datetime, footer, headers, home_button};
use crate::web::routes::player::get_account_url;
use crate::web::TrackingCode;

// TODO: generic response in `crate::web::responses`.
#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum Response {
    Html(Html<String>),
    Redirect(Redirect),
}

#[rocket::get("/search?<query>")]
pub async fn get(
    query: String,
    tracking_code: &State<TrackingCode>,
    api: &State<WargamingApi>,
) -> crate::web::result::Result<Response> {
    clear_user();

    let account_ids: Vec<i32> = api
        .search_accounts(&query)
        .await?
        .iter()
        .map(|account| account.id)
        .collect();
    let mut accounts: Vec<AccountInfo> = api
        .get_account_info(&account_ids)
        .await?
        .into_iter()
        .filter_map(|(_, info)| info)
        .collect();
    if accounts.len() == 1 {
        return Ok(Response::Redirect(Redirect::temporary(get_account_url(
            accounts.first().unwrap().base.id,
        ))));
    }
    accounts.sort_unstable_by(|left, right| {
        right.base.last_battle_time.cmp(&left.base.last_battle_time)
    });

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                (headers())
                title { (query) " – Поиск статистов" }
            }
        }
        body {
            (tracking_code.0)
            nav.navbar.has-shadow.is-fixed-top role="navigation" aria-label="main navigation" {
                div.container {
                    div.navbar-brand {
                        div.navbar-item {
                            div.buttons { (home_button()) }
                        }
                    }
                    div.navbar-menu {
                        div.navbar-end {
                            form.navbar-item action="/search" method="GET" {
                                (account_search("", &query, false))
                            }
                        }
                    }
                }
            }

            section.section."p-0"."m-4" {
                div.container {
                    div.columns.is-centered {
                        div.column."is-6-widescreen"."is-10-tablet" {
                            @if accounts.is_empty() {
                                div.box {
                                    p.content {
                                        "Не найдено ни одного аккаунта с подобным именем."
                                    }
                                    p {
                                        a class="button is-info" href="https://ru.wargaming.net/registration/ru/" {
                                            "Создать аккаунт"
                                        }
                                    }
                                }
                            }
                            @for account in &accounts {
                                div.box {
                                    p.title."is-5" {
                                        a href=(get_account_url(account.base.id)) { (account.base.nickname) }
                                    }
                                    p.subtitle."is-6" {
                                        span.icon-text.has-text-grey {
                                            span { (account.statistics.all.battles) " боев" }
                                            span.icon { i.far.fa-dot-circle {} }
                                            span { (datetime(account.base.last_battle_time, Tense::Past)) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            (footer())
        }
    };

    Ok(Response::Html(Html(markup.into_string())))
}
