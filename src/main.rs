mod logging;
mod opts;
mod wargaming;
mod web;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = opts::parse();
    logging::init(opts.debug)?;
    let _sentry_guard = init_sentry(&opts);
    web::run(&opts.host, opts.port, opts.application_id.clone()).await?;
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
