use tide::{Response, StatusCode};

use crate::database::Database;
use crate::wargaming::WargamingApi;

mod components;
mod middleware;
mod models;
mod partials;
mod responses;
mod views;

#[derive(Clone)]
pub struct State {
    api: WargamingApi,
    database: Database,
}

/// Run the web app.
pub async fn run(
    host: &str,
    port: u16,
    api: WargamingApi,
    database: Database,
) -> anyhow::Result<()> {
    let mut app = tide::with_state(State { api, database });
    app.with(middleware::LoggerMiddleware);
    app.at("/").get(views::index::get);
    app.at("/ru/:account_id").get(views::player::get);
    app.at("/error").get(views::error::get);
    app.at("/site.webmanifest").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/site.webmanifest"),
            "application/json",
        ))
    });
    app.at("/favicon.ico").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/favicon.ico"),
            "image/vnd.microsoft.icon",
        ))
    });
    app.at("/favicon-16x16.png").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/favicon-16x16.png"),
            "image/png",
        ))
    });
    app.at("/favicon-32x32.png").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/favicon-32x32.png"),
            "image/png",
        ))
    });
    app.at("/android-chrome-192x192.png").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/android-chrome-192x192.png"),
            "image/png",
        ))
    });
    app.at("/android-chrome-512x512.png").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/android-chrome-512x512.png"),
            "image/png",
        ))
    });
    app.at("/apple-touch-icon.png").get(|_| async {
        Ok(get_static(
            include_bytes!("web/static/apple-touch-icon.png"),
            "image/png",
        ))
    });
    log::info!("Listening on {}:{}.", host, port);
    app.listen((host, port)).await?;
    Ok(())
}

fn get_static(body: &[u8], content_type: &str) -> Response {
    Response::builder(StatusCode::Ok)
        .body(body)
        .content_type(content_type)
        .header("Cache-Control", "public, max-age=86400, immutable")
        .build()
}
