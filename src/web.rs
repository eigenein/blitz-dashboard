use std::net::IpAddr;
use std::str::FromStr;

use rocket::http::{Status, StatusClass};
use rocket::response::Responder;
use rocket::{response, Response};
use rocket::{routes, Request};
use sentry::integrations::anyhow::capture_anyhow;
use sqlx::PgPool;

use responses::r#static;
use state::State;

use crate::opts::Opts;
use crate::wargaming::WargamingApi;

mod fairings;
mod helpers;
mod index;
mod partials;
mod player;
mod responses;
mod search;
mod state;

type Result<T = ()> = std::result::Result<T, Error>;

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
        .mount("/", routes![index::get])
        .mount("/", routes![search::view::get])
        .mount("/", routes![player::view::get])
        .mount("/", routes![responses::error::get_error])
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

pub struct Error(anyhow::Error);

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let sentry_id = capture_anyhow(&self.0).to_simple().to_string();
        log::error!(
            "{} {}: {:#} (https://sentry.io/eigenein/blitz-dashboard/events/{})",
            request.method(),
            request.uri(),
            self.0,
            sentry_id,
        );
        Response::build()
            .status(Status::InternalServerError)
            .raw_header("x-sentry-id", sentry_id)
            .ok()
    }
}
