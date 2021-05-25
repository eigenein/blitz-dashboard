use crate::web::partials::document;
use maud::{html, Markup};
use tide::http::mime;
use tide::{Response, StatusCode};

/// Wraps the body into a complete HTML document.
pub fn document_response(code: StatusCode, title: Option<&str>, body: Markup) -> tide::Result {
    Ok(Response::builder(code)
        .body(document(title, body).into_string())
        .content_type(mime::HTML)
        .build())
}

pub fn error_document(sentry_id: &sentry::types::Uuid) -> tide::Result {
    document_response(
        StatusCode::InternalServerError,
        Some("Error"),
        html! {
            section class="hero is-fullheight" {
                div class="hero-body" {
                    div class="container" {
                        div class="columns" {
                            div class="column is-6 is-offset-3" {
                                div.card {
                                    header class="card-header" {
                                        p class="card-header-title" { "🤖 Oops!…" }
                                    }
                                    div class="card-content" {
                                        p.content { "Sentry ID: " code { (sentry_id.to_simple()) } "." }
                                        p { a class="button is-info" href="/" { "Go to the Home page" } }
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
