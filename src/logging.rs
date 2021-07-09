use sentry::integrations::log::{LogFilter, SentryLogger};
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

pub fn init(verbosity: i32) -> anyhow::Result<()> {
    let log_level = match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    let logger = TermLogger::new(
        log_level,
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

pub fn clear_user() {
    sentry::configure_scope(|scope| scope.set_user(None));
}

pub fn set_user<U: Into<String>>(username: U) {
    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            username: Some(username.into()),
            ..Default::default()
        }))
    });
}
