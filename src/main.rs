use clap::{crate_name, crate_version};
use futures::try_join;
use sentry::integrations::anyhow::capture_anyhow;

use crate::wargaming::WargamingApi;

mod crawler;
mod database;
mod logging;
mod metrics;
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
    logging::init(opts.verbosity)?;
    log::info!("{} {}", crate_name!(), crate_version!());
    let _sentry_guard = init_sentry(&opts);

    let api = WargamingApi::new(&opts.application_id)?;
    let database = crate::database::open(&opts.database).await?;

    match try_join!(
        web::run(api.clone(), database.clone(), &opts),
        crawler::Crawler::run(api.clone(), database.clone(), opts.crawler),
    ) {
        Ok(_) => Ok(()),
        Err(error) => {
            capture_anyhow(&error);
            Err(error)
        }
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
