use maud::html;
use tide::Redirect;
use tide::StatusCode;

use crate::web::components::account_search;
use crate::web::index::model::IndexViewModel;
use crate::web::player::view::get_account_url;
use crate::web::responses::render_document;
use crate::web::State;

/// Home page that allows searching for a user.
pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = IndexViewModel::new(request).await?;

    if let Some(accounts) = &model.accounts {
        if accounts.len() == 1 {
            return Ok(Redirect::temporary(get_account_url(accounts.first().unwrap().id)).into());
        }
    }

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
                                @if let Some(accounts) = &model.accounts {
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
