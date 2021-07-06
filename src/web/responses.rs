use maud::{html, Markup, DOCTYPE};
use tide::http::mime;
use tide::{Response, StatusCode};

use crate::web::partials::headers;

pub fn html(code: StatusCode, markup: Markup) -> Response {
    Response::builder(code)
        .body(markup.into_string())
        .content_type(mime::HTML)
        .build()
}

pub fn error(sentry_id: &sentry::types::Uuid) -> Response {
    html(
        StatusCode::InternalServerError,
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    (headers(""))
                    title { "Ошибка – Я не статист :(" }
                }
                body {
                    section class="hero is-fullheight" {
                        div class="hero-body" {
                            div class="container" {
                                div class="columns" {
                                    div class="column is-6 is-offset-3" {
                                        div.box {
                                            p.title."is-5" { "Внутренняя ошибка сервера" }
                                            p.content {
                                                "Иногда это происходит из-за ошибки на стороне Wargaming.net."
                                                " Поэтому, можете попробовать обновить страницу."
                                            }
                                            p.content { "В любом случае, отчет уже отправлен разработчикам." }
                                            p.content {
                                                "Вот ссылка на всякий случай: " code { (sentry_id.to_simple()) } "."
                                            }
                                            p { a class="button is-info" href="/" { "Вернуться на главную" } }
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
