use maud::{html, Markup, DOCTYPE};
use tide::http::mime;
use tide::{Response, StatusCode};

mod logging;

pub async fn run(host: &str, port: u16) -> anyhow::Result<()> {
    let mut app = tide::new();
    app.with(tide_compress::CompressMiddleware::new());
    app.with(logging::RequestLogMiddleware);
    app.at("/").get(index);
    log::info!("Listening on {}:{}.", host, port);
    app.listen((host, port)).await?;
    Ok(())
}

async fn index(mut _request: tide::Request<()>) -> tide::Result {
    respond_with_body(html! {
        section class="hero is-fullheight" {
            div class="hero-body" {
                div class="container" {
                    div class="columns" {
                        div class="column is-half is-offset-one-quarter" {
                            form {
                                div class="field has-addons" {
                                    div class="control" {
                                        span class="select is-medium is-rounded" {
                                            select {
                                                option { "🇷🇺 RU" }
                                                option { "🇪🇺 EU" }
                                                option { "🇺🇸 NA" }
                                                option { "🇨🇳 AS" }
                                            }
                                        }
                                    }
                                    div class="control has-icons-left is-expanded" {
                                        input class="input is-medium is-rounded" type="text" placeholder="Username or user ID" autofocus;
                                        span class="icon is-medium is-left" {
                                            i class="fas fa-user" {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn respond_with_body(body: Markup) -> tide::Result {
    let markup: Markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta name="viewport" content="width=device-width, initial-scale=1";
                link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.2/css/bulma.min.css";
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
            }
            body {
                (body)
            }
        }
    };
    Ok(Response::builder(StatusCode::Ok)
        .body(markup.into_string())
        .content_type(mime::HTML)
        .build())
}
