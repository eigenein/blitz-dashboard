use log::LevelFilter;
use sentry::integrations::tracing::EventFilter;
use sentry::ClientInitGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize Sentry.
/// See also: <https://docs.sentry.io/platforms/rust/>.
pub fn init(dsn: &str, verbosity: LevelFilter) -> ClientInitGuard {
    tracing_subscriber::registry()
        .with(sentry::integrations::tracing::layer().event_filter(|_| EventFilter::Breadcrumb))
        .init();
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 0.1,
            debug: [LevelFilter::Trace, LevelFilter::Debug].contains(&verbosity),
            ..Default::default()
        },
    ));
    guard
}
