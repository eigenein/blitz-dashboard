use maud::{html, DOCTYPE};
use poem::web::{Data, Html};
use poem::{handler, IntoResponse, Response};
use tracing::instrument;

use crate::helpers::sentry::clear_user;
use crate::prelude::*;
use crate::wargaming;
use crate::web::partials::{account_search, headers};
use crate::web::TrackingCode;

#[instrument(skip_all)]
#[handler]
pub async fn get(tracking_code: Data<&TrackingCode>) -> Result<Response> {
    clear_user();

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                (headers())
                title { "Я – статист в World of Tanks Blitz!" }
            }
            body {
                (*tracking_code)
                section.hero.is-fullheight {
                    div.hero-body {
                        div.container {
                            div.columns {
                                div.column."is-6"."is-offset-3" {
                                    form action="/search" method="GET" {
                                        div.field.is-grouped.is-grouped-centered.is-grouped-multiline {
                                            p.control {
                                                a.button.is-rounded.is-small href="/ru/103809874" { "Invincible_Beast" }
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
                                        (account_search("is-medium is-rounded", wargaming::Realm::Russia, "", true, false))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(Html(markup.into_string())
        .with_header("Cache-Control", "max-age=604800, stale-while-revalidate=86400")
        .into_response())
}
