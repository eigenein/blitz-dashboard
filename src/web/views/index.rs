use maud::{html, DOCTYPE};
use poem::i18n::Locale;
use poem::web::{Data, Html};
use poem::{handler, IntoResponse};
use tracing::instrument;

use crate::helpers::sentry::clear_user;
use crate::wargaming;
use crate::web::partials::{headers, AccountSearch};
use crate::web::TrackingCode;

#[instrument(skip_all)]
#[handler]
pub async fn get(
    tracking_code: Data<&TrackingCode>,
    locale: Locale,
) -> poem::Result<impl IntoResponse> {
    clear_user();

    let markup = html! {
        (DOCTYPE)
        html lang=(locale.text("html-lang")?) {
            head {
                (headers())
                title { (locale.text("page-title-index")?) }
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
                                        (
                                            AccountSearch::new(wargaming::Realm::Russia, &locale)
                                                .class("is-medium is-rounded")
                                                .has_autofocus(true)
                                                .try_into_markup()?
                                        )
                                        div.field.is-grouped.is-grouped-centered {
                                            p.control {
                                                a.button.is-rounded.is-medium href="/random" {
                                                    span.icon { i.fa-solid.fa-dice {} }
                                                    span { (locale.text("button-feeling-lucky")?) }
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
        .with_header("Cache-Control", "public, max-age=604800, stale-while-revalidate=86400"))
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::web::test::create_standalone_test_client;

    #[tokio::test]
    async fn test_get_ok() -> Result {
        let (_guard, client) = create_standalone_test_client().await?;
        let response = client.get("/").send().await;
        response.assert_status_is_ok();
        response.assert_header_exist("Cache-Control");
        Ok(())
    }
}
