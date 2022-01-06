//! CLI options.

mod parsers;

use std::time::Duration as StdDuration;

use chrono::Duration;
use log::LevelFilter;
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    rename_all = "kebab-case",
    global_settings(&[AppSettings::ColoredHelp, AppSettings::InferSubcommands]),
)]
pub struct Opts {
    /// Sentry DSN
    #[structopt(short, long)]
    pub sentry_dsn: Option<String>,

    /// Increases log verbosity
    #[structopt(
        short = "v",
        long = "verbose",
        parse(from_occurrences = parsers::verbosity),
    )]
    pub verbosity: LevelFilter,

    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(StructOpt)]
pub enum Subcommand {
    Web(WebOpts),

    #[structopt(alias = "crawler")]
    Crawl(CrawlerOpts),

    ImportTankopedia(ImportTankopediaOpts),
    CrawlAccounts(CrawlAccountsOpts),

    #[structopt(alias = "trainer")]
    Train(TrainerOpts),
}

/// Runs the web application
#[derive(StructOpt)]
pub struct WebOpts {
    #[structopt(flatten)]
    pub connections: ConnectionOpts,

    /// Web application bind host
    #[structopt(long, default_value = "::")]
    pub host: String,

    /// Web application bind port
    #[structopt(short, long, default_value = "8081")]
    pub port: u16,

    /// Yandex.Metrika counter number
    #[structopt(long)]
    pub yandex_metrika: Option<String>,

    /// Google Analytics measurement ID
    #[structopt(long)]
    pub gtag: Option<String>,

    /// Disable the caches, do not use on production
    #[structopt(long)]
    pub disable_caches: bool,
}

/// Runs the account crawler
#[derive(StructOpt)]
pub struct CrawlerOpts {
    #[structopt(flatten)]
    pub shared: SharedCrawlerOpts,

    /// Minimum last battle time offset
    #[structopt(long, default_value = "0s", parse(try_from_str = humantime::parse_duration))]
    pub min_offset: StdDuration,

    /// Turn on the automatic minimum last battle offset adjustment based on 50%-lag (experimental)
    #[structopt(long)]
    pub auto_min_offset: bool,

    /// Maximum training stream duration
    #[structopt(long, default_value = "1day", parse(try_from_str = parsers::duration))]
    pub training_stream_duration: Duration,
}

/// Updates the bundled Tankopedia module
#[derive(StructOpt)]
pub struct ImportTankopediaOpts {
    /// Wargaming.net API application ID
    #[structopt(short, long)]
    pub application_id: String,
}

/// Crawls the specified account IDs
#[derive(StructOpt)]
pub struct CrawlAccountsOpts {
    #[structopt(flatten)]
    pub shared: SharedCrawlerOpts,

    /// Starting account ID
    #[structopt(long, parse(try_from_str = parsers::account_id))]
    pub start_id: i32,

    /// Ending account ID (non-inclusive)
    #[structopt(long, parse(try_from_str = parsers::account_id))]
    pub end_id: i32,
}

#[derive(StructOpt)]
pub struct BufferingOpts {
    /// Number of buffered accounts in the stream
    #[structopt(
        long = "n-buffered-accounts",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub n_accounts: usize,

    /// Number of buffered batches in the stream
    #[structopt(
        long = "n-buffered-batches",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub n_batches: usize,
}

#[derive(StructOpt)]
pub struct SharedCrawlerOpts {
    #[structopt(flatten)]
    pub connections: ConnectionOpts,

    #[structopt(flatten)]
    pub buffering: BufferingOpts,

    #[structopt(long, default_value = "100ms", parse(try_from_str = humantime::parse_duration))]
    pub throttling_period: StdDuration,

    /// Metrics logging interval. With `--auto-min-offset` – also the minimum offset update interval
    #[structopt(long, default_value = "1min", parse(try_from_str = humantime::parse_duration))]
    pub log_interval: StdDuration,
}

/// Continuously recalculates the metrics
#[derive(Clone, StructOpt)]
pub struct TrainerOpts {
    /// Redis URI
    #[structopt(long, default_value = "redis://127.0.0.1/0")]
    pub redis_uri: String,

    /// Interval for the recalculation
    #[structopt(
        long,
        default_value = "1minute",
        parse(try_from_str = humantime::parse_duration),
    )]
    pub interval: StdDuration,

    #[structopt(long, default_value = "1hour", parse(try_from_str = parsers::duration))]
    pub time_span: Duration,
}

#[derive(StructOpt)]
pub struct ConnectionOpts {
    #[structopt(flatten)]
    pub internal: InternalConnectionOpts,

    /// Wargaming.net API application ID
    #[structopt(short, long)]
    pub application_id: String,
}

#[derive(StructOpt)]
pub struct InternalConnectionOpts {
    /// PostgreSQL database URI
    #[structopt(short, long = "database")]
    pub database_uri: String,

    /// Initialize the database schema at startup
    #[structopt(long)]
    pub initialize_schema: bool,

    /// Redis URI
    #[structopt(long, default_value = "redis://127.0.0.1/0")]
    pub redis_uri: String,
}
