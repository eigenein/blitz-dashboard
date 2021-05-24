use crate::api::wargaming::WargamingApi;
use crate::web::partials::document;
use maud::Markup;
use tide::http::mime;
use tide::{Response, StatusCode};

mod components;
mod monitoring;
mod partials;
mod views;

#[derive(Clone)]
pub struct State {
    api: WargamingApi,
}

/// Run the web app.
pub async fn run(host: &str, port: u16, application_id: String) -> anyhow::Result<()> {
    let mut app = tide::with_state(State {
        api: WargamingApi::new(application_id),
    });
    app.with(tide_compress::CompressMiddleware::new());
    app.with(monitoring::MonitoringMiddleware);
    app.at("/").get(views::index::get);
    app.at("/ru/:user_id").get(views::user::get);
    app.at("/error").get(views::errors::get);
    log::info!("Listening on {}:{}.", host, port);
    app.listen((host, port)).await?;
    Ok(())
}

/// Wraps the body into a complete HTML document.
pub fn respond_with_body(code: StatusCode, body: Markup) -> tide::Result {
    Ok(Response::builder(code)
        .body(document(body).into_string())
        .content_type(mime::HTML)
        .build())
}
