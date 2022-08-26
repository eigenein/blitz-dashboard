#![warn(
    clippy::all,
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref,
    clippy::map_unwrap_or,
    clippy::explicit_into_iter_loop,
    clippy::unused_self,
    clippy::needless_pass_by_value
)]

use clap::Parser;
use helpers::tracing;
use sentry::integrations::anyhow::capture_anyhow;

use crate::opts::{Opts, Subcommand};
use crate::prelude::*;
use crate::tracing::format_elapsed;

mod crawler;
pub mod database;
mod helpers;
mod math;
mod opts;
mod prelude;
mod tankopedia;
mod trainer;
pub mod wargaming;
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
        Subcommand::Train(opts) => trainer::run(opts).await,
    };
    info!(elapsed = ?start_instant.elapsed(), "the command has finished");
    if let Err(error) = &result {
        capture_anyhow(error);
    }
    result
}
