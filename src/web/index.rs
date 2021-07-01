use maud::{html, DOCTYPE};

use tide::StatusCode;

use crate::web::partials::{account_search, headers};
use crate::web::responses::html;
use crate::web::state::State;

pub async fn get(_: tide::Request<State>) -> tide::Result {
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
