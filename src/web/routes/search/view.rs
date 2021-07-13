use chrono_humanize::Tense;
use maud::{html, DOCTYPE};
use rocket::response::content::Html;
use rocket::response::Redirect;
use rocket::Responder;

use crate::wargaming::AccountSearchCache;
use crate::web::partials::{account_search, datetime, footer, headers};
use crate::web::routes::player::get_account_url;
use crate::web::state::State;

use super::models::ViewModel;

// TODO: generic response in `crate::web::responses`.
#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum Response {
    Html(Html<String>),
    Redirect(Redirect),
}

#[rocket::get("/search?<query>")]
pub async fn get(
    query: &str,
    state: &rocket::State<State>,
    account_search_cache: &rocket::State<AccountSearchCache>,
) -> crate::web::result::Result<Response> {
    let model = ViewModel::new(query.to_string(), &account_search_cache).await?;
    let footer = footer(state).await?;

    if model.accounts.len() == 1 {
        return Ok(Response::Redirect(Redirect::temporary(get_account_url(
            model.accounts.first().unwrap().general.id,
        ))));
    }

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                (headers())
                title { (model.query) " – Поиск статистов" }
            }
        }
        body {
            (state.tracking_code)
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
                                (account_search("", &model.query, false))
                            }
                        }
                    }
                }
            }

            section.section."p-0"."m-4" {
                div.container {
                    div.columns.is-centered {
                        div.column."is-6-widescreen"."is-10-tablet" {
                            @if model.accounts.is_empty() {
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
                            @for account in &model.accounts {
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

            (footer)
        }
    };

    Ok(Response::Html(Html(markup.into_string())))
}
