use crate::opts::{Opts, Subcommand};
use clap::{crate_name, crate_version};
use sentry::integrations::anyhow::capture_anyhow;

mod api;
mod logging;
mod opts;
mod web;

type Result<T = ()> = anyhow::Result<T>;

#[async_std::main]
async fn main() -> crate::Result {
    let opts = opts::parse();
    logging::init(opts.debug)?;
    log::info!("{} {}", crate_name!(), crate_version!());
    let _sentry_guard = init_sentry(&opts);
    let result = run_app(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

async fn run_app(opts: Opts) -> crate::Result {
    match opts.subcommand {
        Subcommand::Web(web_opts) => {
            web::run(&web_opts.host, web_opts.port, opts.application_id.clone()).await
        }
        Subcommand::Crawler(_) => unimplemented!(),
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
