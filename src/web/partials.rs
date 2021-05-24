use crate::web::components::card;
use crate::web::respond_with_document;
use maud::{html, Markup, DOCTYPE};
use tide::StatusCode;

pub fn head(title: Option<&str>) -> Markup {
    html! {
        title { @if let Some(title) = title { (title) " – " } "Blitz Dashboard" }
        meta name="viewport" content="width=device-width, initial-scale=1";
        meta charset="UTF-8";
        link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.2/css/bulma.min.css" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
    }
}

pub fn document(title: Option<&str>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head { (head(title)) }
            body { (body) }
        }
    }
}

/// Renders the error.
pub fn error_document(sentry_id: &sentry::types::Uuid) -> tide::Result {
    respond_with_document(
        StatusCode::InternalServerError,
        Some("Error"),
        html! {
            section class="hero is-fullheight" {
                div class="hero-body" {
                    div class="container" {
                        div class="columns" {
                            div class="column is-6 is-offset-3" {
                                (card(
                                    Some(html! { "🤖 Oops!…" }),
                                    html! {
                                        p.content { "Sentry ID: " code { (sentry_id.to_simple()) } "." }
                                        p { a class="button is-info" href="/" { "Go to the Home page" } }
                                    },
                                ))
                            }
                        }
                    }
                }
            }
        },
    )
}
