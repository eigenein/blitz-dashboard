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
        section class="hero is-info is-fullheight" {
            div class="hero-body" {
                div {
                    p class="title" {
                        "Fullheight hero"
                    }
                    p class="subtitle" {
                        "Fullheight subtitle"
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
