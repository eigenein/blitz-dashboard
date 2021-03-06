use maud::{html, DOCTYPE};
use poem::web::{Data, Html};
use poem::{handler, IntoResponse, Response};
use tracing::instrument;

use crate::helpers::sentry::clear_user;
use crate::wargaming;
use crate::web::partials::{account_search, headers};
use crate::web::TrackingCode;

#[instrument(skip_all)]
#[handler]
pub async fn get(tracking_code: Data<&TrackingCode>) -> poem::Result<Response> {
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
                                                a.button.is-rounded.is-small href="/ru/103809874" { "🇷🇺 Invincible_Beast" }
                                            }
                                            p.control {
                                                a.button.is-rounded.is-small href="/ru/3851977" { "🇷🇺 D_W_S" }
                                            }
                                        }
                                        (account_search("is-medium is-rounded", wargaming::Realm::Russia, "", true, false))
                                        div.field.is-grouped.is-grouped-centered {
                                            p.control {
                                                a.button.is-rounded.is-medium href="/random" {
                                                    span.icon { i.fa-solid.fa-dice {} }
                                                    span { "Мне повезет!" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                script src="https://betteruptime.com/widgets/announcement.js" data-id="144994" async {}
            }
        }
    };

    Ok(Html(markup.into_string())
        .with_header("Cache-Control", "max-age=604800, stale-while-revalidate=86400")
        .into_response())
}
