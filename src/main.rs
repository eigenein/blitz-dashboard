#![warn(clippy::all)]
#![cfg_attr(nightly, feature(test))]

pub use std::result::Result as StdResult;
pub use std::time::Duration as StdDuration;
use std::time::Instant;

use itertools::Itertools;
use sentry::integrations::anyhow::capture_anyhow;
use structopt::StructOpt;
use tracing::info;

use crate::helpers::time::format_elapsed;
use crate::opts::{Opts, Subcommand};

mod aggregator;
mod battle_stream;
mod crawler;
mod database;
mod export_stream;
mod helpers;
mod logging;
mod math;
mod metrics;
mod models;
mod opts;
mod tankopedia;
mod wargaming;
mod web;

#[global_allocator]
static ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

type AHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
type DateTime = chrono::DateTime<chrono::Utc>;
type Result<T = ()> = anyhow::Result<T>;

#[tokio::main]
#[tracing::instrument]
async fn main() -> crate::Result {
    let opts = Opts::from_args();
    logging::init(opts.verbosity)?;
    info!(
        version = CRATE_VERSION,
        args = std::env::args().skip(1).join(" ").as_str(),
    );
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
        Subcommand::Aggregate(opts) => aggregator::run(opts).await,
        Subcommand::Crawl(opts) => crawler::run_crawler(opts).await,
        Subcommand::CrawlAccounts(opts) => crawler::crawl_accounts(opts).await,
        Subcommand::ExportStream(opts) => export_stream::run(opts).await,
        Subcommand::ImportTankopedia(opts) => tankopedia::import(opts).await,
        Subcommand::Web(opts) => web::run(opts).await,
    };
    info!(
        elapsed = %format_elapsed(&start_instant),
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
                debug: false,
                ..Default::default()
            },
        ))
    })
}
