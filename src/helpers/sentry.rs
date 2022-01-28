use log::LevelFilter;
use sentry::integrations::tracing::EventFilter;
use sentry::ClientInitGuard;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize Sentry.
/// See also: <https://docs.sentry.io/platforms/rust/>.
pub fn init(dsn: &str, verbosity: LevelFilter, traces_sample_rate: f32) -> ClientInitGuard {
    tracing_subscriber::registry()
        .with(
            sentry::integrations::tracing::layer()
                .event_filter(|_| EventFilter::Breadcrumb)
                .span_filter(|metadata| metadata.level() != &Level::TRACE),
        )
        .init();
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate,
            debug: [LevelFilter::Trace, LevelFilter::Debug].contains(&verbosity),
            ..Default::default()
        },
    ));
    guard
}

/// Clears current user in Sentry.
pub fn clear_user() {
    sentry::configure_scope(|scope| scope.set_user(None));
}

/// Sets current user in Sentry.
pub fn set_user<U: Into<String>>(username: U) {
    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            username: Some(username.into()),
            ..Default::default()
        }))
    });
}
