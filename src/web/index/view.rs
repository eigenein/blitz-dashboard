use maud::{html, DOCTYPE};
use tide::Redirect;
use tide::StatusCode;

use crate::web::index::model::IndexViewModel;
use crate::web::partials::{account_search, headers};
use crate::web::player::view::get_account_url;
use crate::web::responses::html;
use crate::web::state::State;

/// Home page that allows searching for a user.
pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = IndexViewModel::new(&request).await?;

    if let Some(accounts) = &model.accounts {
        if accounts.len() == 1 {
            return Ok(Redirect::temporary(get_account_url(accounts.first().unwrap().id)).into());
        }
    }

    Ok(html(
        StatusCode::Ok,
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    (headers())
                    title { "Я статист!" }
                }
                body {
                    section class="hero is-fullheight" {
                        div class="hero-body" {
                            div class="container" {
                                div class="columns" {
                                    div class="column is-8 is-offset-2" {
                                        form action="/" method="GET" {
                                            (account_search("is-medium", "", true))
                                        }
                                        @if let Some(accounts) = &model.accounts {
                                            div class="buttons mt-4" {
                                                @for account in accounts.iter() {
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
                }
            }
        },
    ))
}
