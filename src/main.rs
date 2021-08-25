#![warn(clippy::all)]

pub use std::time::Duration as StdDuration;

use log::{Level, LevelFilter};
use sentry::integrations::anyhow::capture_anyhow;
use structopt::StructOpt;

use crate::metrics::Stopwatch;
use crate::opts::{Opts, Subcommand};

mod backoff;
mod crawler;
mod database;
mod logging;
mod metrics;
mod miniz;
mod models;
mod opts;
mod redis;
mod serde;
mod statistics;
mod tankopedia;
mod time;
mod wargaming;
mod web;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

type Result<T = ()> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> crate::Result {
    let opts = Opts::from_args();
    logging::init(opts.verbosity)?;
    log::info!("{} {}", CRATE_NAME, CRATE_VERSION);
    let _sentry_guard = init_sentry(&opts);

    let result = run_subcommand(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

async fn run_subcommand(opts: Opts) -> crate::Result {
    let _stopwatch = Stopwatch::new("The subcommand has finished").level(Level::Info);
    match opts.subcommand {
        Subcommand::Web(opts) => web::run(opts).await,
        Subcommand::Crawler(opts) => crawler::run_crawler(opts).await,
        Subcommand::ImportTankopedia(opts) => tankopedia::import(opts).await,
        Subcommand::CrawlAccounts(opts) => crawler::crawl_accounts(opts).await,
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
                debug: [LevelFilter::Trace, LevelFilter::Debug, LevelFilter::Info]
                    .contains(&opts.verbosity),
                ..Default::default()
            },
        ))
    })
}
