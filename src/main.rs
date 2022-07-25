#![warn(clippy::all)]

use clap::Parser;
use helpers::tracing;

use crate::opts::{Opts, Subcommand};
use crate::prelude::*;
use crate::tracing::format_elapsed;

mod crawler;
mod database;
mod helpers;
mod math;
mod opts;
mod prelude;
mod tankopedia;
mod wargaming;
mod web;

const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result {
    let opts: Opts = Opts::parse();
    let _sentry_guard = tracing::init(opts.sentry_dsn.clone(), opts.traces_sample_rate)?;
    info!(version = CRATE_VERSION);

    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?
        .block_on(run_subcommand(opts))
}

async fn run_subcommand(opts: Opts) -> Result {
    let start_instant = Instant::now();
    let result = match opts.subcommand {
        Subcommand::Crawl(opts) => crawler::run_crawler(opts).await,
        Subcommand::CrawlAccounts(opts) => crawler::crawl_accounts(opts).await,
        Subcommand::ImportTankopedia(opts) => tankopedia::import(opts).await,
        Subcommand::Web(opts) => web::run(opts).await,
    };
    info!(elapsed = format_elapsed(start_instant).as_str());
    result
}
