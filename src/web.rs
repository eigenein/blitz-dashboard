use crate::api::wargaming::WargamingApi;
use mongodb::Database;

mod components;
mod middleware;
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
    app.with(middleware::LoggingMiddleware);
    app.at("/").get(views::index::get);
    app.at("/ru/:account_id").get(views::player::get);
    app.at("/error").get(views::errors::get);
    log::info!("Listening on {}:{}.", host, port);
    app.listen((host, port)).await?;
    Ok(())
}
