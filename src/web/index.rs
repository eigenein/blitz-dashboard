use maud::{html, DOCTYPE};
use rocket::response::content;
use rocket::response::content::Html;

use crate::logging::clear_user;
use crate::web::partials::{account_search, headers};
use crate::web::state::State;

#[rocket::get("/")]
pub async fn get(state: &rocket::State<State>) -> super::result::Result<Html<String>> {
    clear_user();

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                title { "Я статист!" }
            }
            body {
                (state.tracking_code)
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
                                            p.control {
                                                a.button.is-rounded.is-small href="/ru/123484971" { "Chunya_Dobryak" }
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
