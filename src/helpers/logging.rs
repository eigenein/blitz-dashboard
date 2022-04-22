use chrono::Local;
use std::io::Write;

use log::{Level, LevelFilter, Log, Metadata, Record};

/// Initialises logging.
pub fn init(max_level: LevelFilter, no_journald: bool) -> anyhow::Result<()> {
    log::set_boxed_logger(Box::new(JournaldLogger { no_journald }))?;
    log::set_max_level(max_level);
    Ok(())
}

struct JournaldLogger {
    no_journald: bool,
}

impl Log for JournaldLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.target().starts_with("blitz_dashboard")
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let target = record.target();
            if !self.no_journald {
                eprintln!(
                    "{}{} ({}) {}\u{001b}[0m",
                    journald_prefix(record.level()),
                    level_prefix(record.level()),
                    target.strip_prefix("blitz_dashboard::").unwrap_or(target),
                    record.args(),
                );
            } else {
                eprintln!(
                    "{} {} ({}) {}\u{001b}[0m",
                    Local::now().format("%b %d %_H:%M:%S.%3f"),
                    level_prefix(record.level()),
                    target.strip_prefix("blitz_dashboard::").unwrap_or(target),
                    record.args(),
                );
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

fn journald_prefix(level: Level) -> &'static str {
    match level {
        Level::Trace => "<7>",
        Level::Debug => "<6>",
        Level::Info => "<5>",
        Level::Warn => "<4>",
        Level::Error => "<3>",
    }
}

fn level_prefix(level: Level) -> &'static str {
    match level {
        Level::Trace => "[T]",
        Level::Debug => "[D]",
        Level::Info => "\u{001b}[32m[I]",
        Level::Warn => "\u{001b}[33;1m[W]",
        Level::Error => "\u{001b}[31;1m[E]",
    }
}
