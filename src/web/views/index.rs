use crate::web::views::user::get_user_url;
use crate::web::{respond_with_body, State};
use maud::html;
use serde::Deserialize;
use tide::StatusCode;

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

    let accounts = if query.search.len() >= 3 {
        Some(state.api.search_accounts(&query.search).await?)
    } else {
        None
    };

    respond_with_body(
        StatusCode::Ok,
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
                                            input class="input is-medium is-rounded" type="text" value=(query.search) name="search" placeholder="Username or user ID" autocomplete="nickname" pattern="\\w+" minlength="3" maxlength="24" autofocus required;
                                            span class="icon is-medium is-left" {
                                                i class="fas fa-user" {}
                                            }
                                        }
                                        div class="control" {
                                            input class="button is-medium is-rounded is-info" type="submit" value="Search";
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
