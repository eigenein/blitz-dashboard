use maud::{html, DOCTYPE};
use tide::StatusCode;

use crate::logging::clear_user;
use crate::web::partials::{account_search, headers};
use crate::web::responses::html;
use crate::web::state::State;

pub async fn get(request: tide::Request<State>) -> tide::Result {
    clear_user();

    Ok(html(
        StatusCode::Ok,
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    (headers(request.state().yandex_metrika.as_deref()))
                    title { "Я статист!" }
                }
                body {
                    section class="hero is-fullheight" {
                        div class="hero-body" {
                            div class="container" {
                                div class="columns" {
                                    div class="column is-8 is-offset-2" {
                                        form action="/search" method="GET" {
                                            (account_search("is-medium", "", true))
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
