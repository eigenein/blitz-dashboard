use log::{Level, LevelFilter, Log, Metadata, Record};

use sentry::integrations::log::{LogFilter, SentryLogger};
use std::io::Write;

/// Initialises logging.
pub fn init(verbosity: i32) -> anyhow::Result<()> {
    log::set_boxed_logger(Box::new(
        SentryLogger::with_dest(JournaldLogger).filter(|_| LogFilter::Breadcrumb),
    ))?;
    log::set_max_level(convert_verbosity_to_level(verbosity));
    Ok(())
}

struct JournaldLogger;

impl Log for JournaldLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.target().starts_with("blitz_dashboard")
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "{}{}\u{001b}[0m",
                convert_level_to_prefix(record.level()),
                record.args(),
            );
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

fn convert_verbosity_to_level(verbosity: i32) -> LevelFilter {
    match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

fn convert_level_to_prefix(level: Level) -> &'static str {
    match level {
        Level::Trace => "<7>\x08\x08\x08[T] ",
        Level::Debug => "<6>\x08\x08\x08[D] ",
        Level::Info => "<5>\x08\x08\x08\u{001b}[32m[I] ",
        Level::Warn => "<4>\x08\x08\x08\u{001b}[33;1m[W] ",
        Level::Error => "<3>\x08\x08\x08\u{001b}[31;1m[E] ",
    }
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
