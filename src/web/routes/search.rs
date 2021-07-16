use chrono_humanize::Tense;
use maud::{html, DOCTYPE};
use rocket::response::content::Html;
use rocket::response::Redirect;
use rocket::{Responder, State};

use crate::logging::clear_user;
use crate::wargaming::cache::account::search::AccountSearchCache;
use crate::web::partials::{account_search, datetime, footer, headers};
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
    account_search_cache: &State<AccountSearchCache>,
    tracking_code: &State<TrackingCode>,
) -> crate::web::result::Result<Response> {
    clear_user();

    let mut accounts = account_search_cache.get(&query).await?.to_vec();
    if accounts.len() == 1 {
        return Ok(Response::Redirect(Redirect::temporary(get_account_url(
            accounts.first().unwrap().general.id,
        ))));
    }
    accounts.sort_unstable_by(|left, right| {
        right
            .general
            .last_battle_time
            .cmp(&left.general.last_battle_time)
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
                            div.buttons {
                                a.button.is-link.is-rounded href="/" {
                                    span.icon { i.fas.fa-home {} }
                                    span { "На главную" }
                                }
                            }
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
                                        a href=(get_account_url(account.general.id)) { (account.general.nickname) }
                                    }
                                    p.subtitle."is-6" {
                                        span.icon-text.has-text-grey {
                                            span { (account.statistics.all.battles) " боев" }
                                            span.icon { i.far.fa-dot-circle {} }
                                            span { (datetime(account.general.last_battle_time, Tense::Past)) }
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
