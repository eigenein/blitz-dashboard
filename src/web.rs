use maud::html;
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
    let markup: maud::Markup = html! { "Hello" };
    Ok(Response::builder(StatusCode::Ok)
        .body(markup.into_string())
        .content_type(mime::HTML)
        .build())
}
