#![warn(clippy::all)]

use clap::{crate_name, crate_version};
use sentry::integrations::anyhow::capture_anyhow;

use wargaming::WargamingApi;

use crate::opts::{Opts, Subcommand};

mod crawler;
mod database;
mod logging;
mod metrics;
mod models;
mod opts;
mod serde;
mod statistics;
mod tankopedia;
mod time;
mod wargaming;
mod web;

type Result<T = ()> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> crate::Result {
    let opts = opts::parse();
    logging::init(opts.verbosity)?;
    log::info!("{} {}", crate_name!(), crate_version!());
    let _sentry_guard = init_sentry(&opts);

    let result = run_subcommand(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

async fn run_subcommand(opts: Opts) -> crate::Result {
    let api = WargamingApi::new(&opts.application_id)?;
    let database = crate::database::open(&opts.database).await?;

    match opts.subcommand {
        Subcommand::Web(opts) => web::run(api, database, opts).await,
        Subcommand::Crawler(_) => crawler::Crawler::run(api, database).await,
    }
}

/// Initialize Sentry.
/// See also: <https://docs.sentry.io/platforms/rust/>.
fn init_sentry(opts: &opts::Opts) -> Option<sentry::ClientInitGuard> {
    opts.sentry_dsn.as_ref().map(|dsn| {
        sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                debug: opts.verbosity != 0,
                ..Default::default()
            },
        ))
    })
}
