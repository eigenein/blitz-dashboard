#![warn(clippy::all)]
#![cfg_attr(nightly, feature(test))]

pub use std::time::Duration as StdDuration;
use std::time::Instant;

use itertools::Itertools;
use log::LevelFilter;
use sentry::integrations::anyhow::capture_anyhow;
use structopt::StructOpt;

use crate::helpers::format_elapsed;
use crate::opts::{Opts, Subcommand};

mod backoff;
mod crawler;
mod database;
mod helpers;
mod logging;
mod math;
mod metrics;
mod models;
mod opts;
mod tankopedia;
mod trainer;
mod wargaming;
mod web;

#[global_allocator]
static ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

type Result<T = ()> = anyhow::Result<T>;
type Vector = Vec<f64>;
type DateTime = chrono::DateTime<chrono::Utc>;

#[tokio::main]
#[tracing::instrument]
async fn main() -> crate::Result {
    let opts = Opts::from_args();
    logging::init(opts.verbosity)?;
    log::info!("{} {}", CRATE_NAME, CRATE_VERSION);
    log::info!("started with: {}", std::env::args().skip(1).join(" "));
    let _sentry_guard = init_sentry(&opts);

    let result = run_subcommand(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

#[tracing::instrument(skip_all)]
async fn run_subcommand(opts: Opts) -> crate::Result {
    let start_instant = Instant::now();
    let result = match opts.subcommand {
        Subcommand::CrawlAccounts(opts) => crawler::crawl_accounts(opts).await,
        Subcommand::Crawl(opts) => crawler::run_crawler(opts).await,
        Subcommand::ImportTankopedia(opts) => tankopedia::import(opts).await,
        Subcommand::Train(opts) => trainer::run(opts).await,
        Subcommand::Web(opts) => web::run(opts).await,
    };
    tracing::info!(
        elapsed = format_elapsed(&start_instant).as_str(),
        "finished",
    );
    result
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
