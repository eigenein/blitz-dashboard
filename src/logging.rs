use std::borrow::Borrow;

use sentry::integrations::anyhow::capture_anyhow;
use sentry::integrations::log::{LogFilter, SentryLogger};
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

pub fn init(debug: bool) -> anyhow::Result<()> {
    let logger = TermLogger::new(
        if !debug {
            LevelFilter::Info
        } else {
            LevelFilter::Debug
        },
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Off)
            .set_location_level(LevelFilter::Off)
            .set_time_level(LevelFilter::Off)
            .add_filter_allow_str("blitz_dashboard")
            .set_thread_level(LevelFilter::Off)
            .build(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    );
    log::set_boxed_logger(Box::new(
        SentryLogger::with_dest(logger).filter(|_| LogFilter::Breadcrumb),
    ))?;
    log::set_max_level(LevelFilter::Debug);
    Ok(())
}

/// Check the result and log an error, if any.
#[allow(dead_code)]
pub fn log_anyhow<T, R: Borrow<crate::Result<T>>>(result: R) {
    if let Err(ref error) = result.borrow() {
        log::error!(
            "{:#} (https://sentry.io/eigenein/blitz-dashboard/events/{})",
            error,
            capture_anyhow(error).to_simple()
        );
    }
}
