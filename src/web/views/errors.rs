use crate::web::components::card;
use crate::web::utils::respond_with_body;
use crate::web::State;
use maud::html;
use tide::{Request, StatusCode};

/// Debug endpoint that always returns an error.
pub async fn get(_request: Request<State>) -> tide::Result {
    Err(tide::Error::from_str(
        StatusCode::InternalServerError,
        "This is a simulated error",
    ))
}

/// Renders the error.
pub fn get_error_view(sentry_id: &sentry::types::Uuid) -> tide::Result {
    respond_with_body(
        StatusCode::InternalServerError,
        html! {
            section class="hero is-fullheight" {
                div class="hero-body" {
                    div class="container" {
                        div class="columns" {
                            div class="column is-4 is-offset-4" {
                                (card(
                                    Some(html! { "🤖 Oops!…" }),
                                    Some(html! { { "Sentry ID: " code { (sentry_id.to_simple()) } "." } }),
                                ))
                            }
                        }
                    }
                }
            }
        },
    )
}
