use crate::web::components::{account_search, SEARCH_QUERY_LENGTH};
use crate::web::responses::render_document;
use crate::web::views::player::get_account_url;
use crate::web::State;
use maud::html;
use serde::Deserialize;
use tide::{Redirect, StatusCode};

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

    let accounts = if SEARCH_QUERY_LENGTH.contains(&query.search.len()) {
        let accounts = state.api.search_accounts(&query.search).await?;
        if accounts.len() == 1 {
            return Ok(Redirect::temporary(get_account_url(accounts.first().unwrap().id)).into());
        }
        Some(accounts)
    } else {
        None
    };

    Ok(render_document(
        StatusCode::Ok,
        None,
        html! {
            section class="hero is-fullheight" {
                div class="hero-body" {
                    div class="container" {
                        div class="columns" {
                            div class="column is-8 is-offset-2" {
                                form action="/" method="GET" {
                                    (account_search("is-medium", true))
                                }
                                @if let Some(accounts) = accounts {
                                    div class="buttons mt-4" {
                                        @for account in accounts {
                                            a class="button is-link is-small is-rounded" href=(get_account_url(account.id)) {
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
    ))
}
