use maud::{html, DOCTYPE};
use rocket::response::content;
use rocket::response::content::Html;
use rocket::State;

use crate::logging::clear_user;
use crate::web::partials::{account_search, headers};
use crate::web::TrackingCode;

#[rocket::get("/")]
pub async fn get(tracking_code: &State<TrackingCode>) -> crate::web::result::Result<Html<String>> {
    clear_user();

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                title { "Я статист!" }
            }
            body {
                (tracking_code.0)
                section.hero.is-fullheight {
                    div.hero-body {
                        div.container {
                            div.columns {
                                div.column."is-8"."is-offset-2" {
                                    form action="/search" method="GET" {
                                        div.field.is-grouped.is-grouped-centered.is-grouped-multiline {
                                            div.control {
                                                div.buttons.has-addons.is-small.is-rounded {
                                                    a.button.is-rounded.is-small href="/ru/103809874" { "Invincible_Beast" }
                                                    a.button.is-rounded.is-small href="/ru/133054164" { "Lucky_Vikk" }
                                                }
                                            }
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
                                        }
                                        (account_search("is-medium", "", true, false))
                                        div.field.is-grouped.is-grouped-centered {
                                            p.control {
                                                a.button.is-link.is-rounded.is-medium href="/random" { "Мне повезет!" }
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
    };

    Ok(content::Html(markup.into_string()))
}
