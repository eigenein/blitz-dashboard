use maud::{html, Markup};
use tide::http::mime;
use tide::{Response, StatusCode};

use crate::web::partials::document;

/// Wraps the body into a complete HTML document.
pub fn render_document(code: StatusCode, title: Option<&str>, body: Markup) -> Response {
    Response::builder(code)
        .body(document(title, body).into_string())
        .content_type(mime::HTML)
        .build()
}

pub fn render_error(sentry_id: &sentry::types::Uuid) -> Response {
    render_document(
        StatusCode::InternalServerError,
        Some("Error"),
        html! {
            section class="hero is-fullheight" {
                div class="hero-body" {
                    div class="container" {
                        div class="columns" {
                            div class="column is-6 is-offset-3" {
                                div.box {
                                    p.title."is-5" { "Internal error" }
                                    p.content {
                                        "Sometimes this happens because of a Wargaming.net error."
                                        " So, you may try to refresh the page."
                                    }
                                    p.content { "Anyway, the error is already reported." }
                                    p.content {
                                        "Here is the reference: " code { (sentry_id.to_simple()) } "."
                                    }
                                    p { a class="button is-info" href="/" { "Go to the Home page" } }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}
