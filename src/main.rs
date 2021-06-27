use clap::{crate_name, crate_version};
use sentry::integrations::anyhow::capture_anyhow;

use crate::database::Database;
use crate::opts::{Opts, Subcommand};
use crate::wargaming::WargamingApi;

mod crawler;
mod database;
mod logging;
mod models;
mod opts;
mod serde;
mod statistics;
mod tankopedia;
mod wargaming;
mod web;

type Result<T = ()> = anyhow::Result<T>;

#[async_std::main]
async fn main() -> crate::Result {
    let opts = opts::parse();
    logging::init(opts.debug)?;
    log::info!("{} {}", crate_name!(), crate_version!());
    let _sentry_guard = init_sentry(&opts);
    let result = run_subcommand(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

async fn run_subcommand(opts: Opts) -> crate::Result {
    let api = WargamingApi::new(&opts.application_id);
    let database = Database::open(opts.database)?;
    match opts.subcommand {
        Subcommand::Web(web_opts) => web::run(&web_opts.host, web_opts.port, api, database).await,
        Subcommand::Crawler(crawler_opts) => crawler::run(api, database, crawler_opts.once).await,
        Subcommand::ImportTankopedia(_) => tankopedia::run(api, database).await,
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
                debug: opts.debug,
                ..Default::default()
            },
        ))
    })
}
