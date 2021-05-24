use crate::web::views::user::get_user_url;
use crate::web::{respond_with_document, State};
use maud::html;
use serde::Deserialize;
use std::ops::Range;
use tide::StatusCode;

const MIN_QUERY_LENGTH: usize = 3;
const MAX_QUERY_LENGTH: usize = 24;
const QUERY_LENGTH: Range<usize> = MIN_QUERY_LENGTH..(MAX_QUERY_LENGTH + 1);

/// User search query.
#[derive(Deserialize)]
struct QueryString {
    #[serde(default = "String::default")]
    search: String,
}

/// Home page that allows searching for a user.
pub async fn get(request: tide::Request<State>) -> tide::Result {
    let query: QueryString = request.query()?;
    let state = request.state();

    let accounts = if QUERY_LENGTH.contains(&query.search.len()) {
        Some(state.api.search_accounts(&query.search).await?)
    } else {
        None
    };

    respond_with_document(
        StatusCode::Ok,
        None,
        html! {
            section class="hero is-fullheight" {
                div class="hero-body" {
                    div class="container" {
                        div class="columns" {
                            div class="column is-8 is-offset-2" {
                                form action="/" method="GET" {
                                    div class="field has-addons" {
                                        div class="control" {
                                            span class="select is-medium is-rounded" {
                                                select disabled {
                                                    option { "🇷🇺 RU" }
                                                    option { "🇪🇺 EU" }
                                                    option { "🇺🇸 NA" }
                                                    option { "🇨🇳 AS" }
                                                }
                                            }
                                        }
                                        div class="control has-icons-left is-expanded" {
                                            input
                                                class="input is-medium is-rounded"
                                                type="text"
                                                value=(query.search)
                                                name="search"
                                                placeholder="Wargaming.net username"
                                                autocomplete="nickname"
                                                pattern="\\w+"
                                                minlength=(MIN_QUERY_LENGTH)
                                                maxlength=(MAX_QUERY_LENGTH)
                                                autofocus
                                                required;
                                            span class="icon is-medium is-left" {
                                                i class="fas fa-user" {}
                                            }
                                        }
                                        div class="control" {
                                            input class="button is-medium is-rounded is-link" type="submit" value="Search";
                                        }
                                    }
                                }
                                @if let Some(accounts) = accounts {
                                    div class="buttons mt-4" {
                                        @for account in accounts {
                                            a class="button is-link is-small is-rounded" href=(get_user_url(account.id)) {
                                                span.icon { i class="fas fa-user" {} }
                                                span { (account.nickname) }
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
