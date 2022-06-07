use std::borrow::Cow;

use sentry::integrations::tracing::EventFilter;
use sentry::{ClientInitGuard, ClientOptions};
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::prelude::*;

/// Initialises tracing.
pub fn init(sentry_dsn: Option<String>, traces_sample_rate: f32) -> Result<ClientInitGuard> {
    let guard = sentry::init((
        sentry_dsn,
        ClientOptions {
            release: Some(Cow::Borrowed(env!("CARGO_PKG_VERSION"))),
            traces_sample_rate,
            ..Default::default()
        },
    ));

    let sentry_filter = EnvFilter::try_from_env("BLITZ_DASHBOARD_SENTRY_LOG")
        .or_else(|_| EnvFilter::try_new("blitz_dashboard=trace"))?;
    let sentry_layer = sentry::integrations::tracing::layer()
        .event_filter(|metadata| match metadata.level() {
            &Level::ERROR | &Level::WARN => EventFilter::Event,
            &Level::INFO | &Level::DEBUG | &Level::TRACE => EventFilter::Breadcrumb,
        })
        .span_filter(|metadata| {
            matches!(metadata.level(), &Level::ERROR | &Level::WARN | &Level::INFO | &Level::DEBUG)
        })
        .with_filter(sentry_filter);

    let format_filter = EnvFilter::try_from_env("BLITZ_DASHBOARD_LOG")
        .or_else(|_| EnvFilter::try_new("blitz_dashboard=info"))?;
    let format_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_filter(format_filter);

    tracing_subscriber::Registry::default()
        .with(sentry_layer)
        .with(format_layer)
        .init();

    Ok(guard)
}

pub fn format_duration(duration: StdDuration) -> String {
    humantime::format_duration(duration).to_string()
}

pub fn format_elapsed(instant: Instant) -> String {
    format_duration(instant.elapsed())
}
