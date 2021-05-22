use crate::wargaming::WargamingApi;

mod components;
mod monitoring;
mod utils;
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
    app.at("/error").get(views::errors::get);
    log::info!("Listening on {}:{}.", host, port);
    app.listen((host, port)).await?;
    Ok(())
}
