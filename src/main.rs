#![warn(clippy::all)]
#![cfg_attr(nightly, feature(test))]

use helpers::tracing;
use itertools::Itertools;
use structopt::StructOpt;

use crate::opts::{Opts, Subcommand};
use crate::prelude::*;

mod crawler;
mod database;
mod helpers;
mod math;
mod opts;
mod prelude;
mod tankopedia;
mod wargaming;
mod web;

#[global_allocator]
static ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result {
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(4 * 1024 * 1024)
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result {
    let opts = Opts::from_args();
    let _sentry_guard = crate::tracing::init(opts.sentry_dsn.clone(), opts.traces_sample_rate)?;
    info!(version = CRATE_VERSION, args = std::env::args().skip(1).join(" ").as_str());

    let result = run_subcommand(opts).await;
    if let Err(error) = &result {
        error!("fatal error: {:#}", error);
    }
    result
}

async fn run_subcommand(opts: Opts) -> Result {
    let start_instant = Instant::now();
    let result = match opts.subcommand {
        Subcommand::Crawl(opts) => crawler::run_crawler(opts).await,
        Subcommand::CrawlAccounts(opts) => crawler::crawl_accounts(opts).await,
        Subcommand::ImportTankopedia(opts) => tankopedia::import(opts).await,
        Subcommand::Web(opts) => web::run(opts).await,
        Subcommand::InitializeDatabase(opts) => {
            database::open(&opts.database_uri, true).await?;
            Ok(())
        }
    };
    info!(elapsed = ?start_instant.elapsed(), "finished");
    result
}
