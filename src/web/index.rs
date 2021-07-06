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
                    (headers(&request.state().extra_html_headers))
                    title { "Я статист!" }
                }
                body {
                    section.hero.is-fullheight {
                        div.hero-body {
                            div.container {
                                div.columns {
                                    div.column."is-8"."is-offset-2" {
                                        form action="/search" method="GET" {
                                            (account_search("is-medium", "", true))
                                            div.field.is-grouped.is-grouped-centered.is-grouped-multiline {
                                                p.control {
                                                    a.button.is-rounded.is-small href="/ru/3851977" { "D_W_S" }
                                                }
                                                p.control {
                                                    a.button.is-rounded.is-small href="/ru/5303075" { "Perfect_M1nd" }
                                                }
                                                p.control {
                                                    a.button.is-rounded.is-small href="/ru/4435872" { "_n0_skill_just_luck_" }
                                                }
                                                p.control {
                                                    a.button.is-rounded.is-small href="/ru/2992069" { "Tortik" }
                                                }
                                                p.control {
                                                    a.button.is-rounded.is-small href="/ru/103809874" { "Invincible_Beast" }
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
