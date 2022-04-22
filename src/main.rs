#![warn(clippy::all)]
#![cfg_attr(nightly, feature(test))]

pub use std::result::Result as StdResult;
pub use std::time::Duration as StdDuration;
use std::time::Instant;

use helpers::logging;
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
mod math;
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
    logging::init(opts.verbosity, opts.no_journald)?;
    info!(
        version = CRATE_VERSION,
        args = std::env::args().skip(1).join(" ").as_str(),
    );
    let _sentry_guard = opts
        .sentry_dsn
        .as_ref()
        .map(|dsn| crate::helpers::sentry::init(dsn, opts.verbosity, opts.traces_sample_rate));

    let result = run_subcommand(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

async fn run_subcommand(opts: Opts) -> crate::Result {
    let start_instant = Instant::now();
    let result = match opts.subcommand {
        Subcommand::Aggregate(opts) => aggregator::run(opts).await,
        Subcommand::Crawl(opts) => crawler::run_crawler(opts).await,
        Subcommand::CrawlAccounts(opts) => crawler::crawl_accounts(opts).await,
        Subcommand::ExportStream(opts) => export_stream::run(opts).await,
        Subcommand::ImportTankopedia(opts) => tankopedia::import(opts).await,
        Subcommand::Web(opts) => web::run(opts).await,
        Subcommand::InitializeDatabase(opts) => {
            database::open(&opts.database_uri, true).await?;
            Ok(())
        }
    };
    info!(
        elapsed = %format_elapsed(&start_instant),
        "finished",
    );
    result
}
