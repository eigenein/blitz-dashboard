use std::net::IpAddr;
use std::str::FromStr;

use rocket::http::{Status, StatusClass};
use rocket::response;
use rocket::{routes, Request};
use sqlx::PgPool;

use routes::r#static;
use state::State;

use crate::opts::Opts;
use crate::wargaming::WargamingApi;

mod error;
mod fairings;
mod helpers;
mod partials;
mod player;
mod result;
mod routes;
mod search;
mod state;

/// Run the web app.
pub async fn run(api: WargamingApi, database: PgPool, opts: &Opts) -> crate::Result {
    if !opts.web {
        return Ok(());
    }

    log::info!("Listening on {}:{}.", opts.host, opts.port);
    rocket::custom(to_config(&opts)?)
        .manage(State::new(api, database, opts).await?)
        .mount("/", routes![r#static::get_site_manifest])
        .mount("/", routes![r#static::get_favicon])
        .mount("/", routes![r#static::get_favicon_16x16])
        .mount("/", routes![r#static::get_favicon_32x32])
        .mount("/", routes![r#static::get_android_chrome_192x192])
        .mount("/", routes![r#static::get_android_chrome_512x512])
        .mount("/", routes![r#static::get_apple_touch_icon])
        .mount("/", routes![routes::index::get])
        .mount("/", routes![search::view::get])
        .mount("/", routes![player::view::get])
        .mount("/", routes![routes::error::get_error])
        .register("/", rocket::catchers![default_catcher])
        .attach(fairings::SecurityHeaders)
        .launch()
        .await?;
    Ok(())
}

#[rocket::catch(default)]
fn default_catcher(status: Status, request: &Request<'_>) -> response::status::Custom<()> {
    match status.class() {
        StatusClass::ClientError => {
            log::warn!("{} {}: {}", request.method(), request.uri(), status);
        }
        StatusClass::ServerError => {
            log::error!("{} {}: {}", request.method(), request.uri(), status);
        }
        _ => {}
    }
    response::status::Custom(status, ())
}

fn to_config(opts: &Opts) -> crate::Result<rocket::Config> {
    Ok(rocket::Config {
        address: IpAddr::from_str(&opts.host)?,
        port: opts.port,
        log_level: rocket::log::LogLevel::Off,
        ..Default::default()
    })
}
