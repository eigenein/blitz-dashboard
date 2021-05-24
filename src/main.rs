use crate::opts::Subcommand;

mod api;
mod logging;
mod opts;
mod web;

type Result<T = ()> = anyhow::Result<T>;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = opts::parse();
    logging::init(opts.debug)?;
    let _sentry_guard = init_sentry(&opts);
    match opts.subcommand {
        Subcommand::Web(web_opts) => {
            web::run(&web_opts.host, web_opts.port, opts.application_id.clone()).await?
        }
        Subcommand::Crawler(_) => unimplemented!(),
    }
    Ok(())
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
