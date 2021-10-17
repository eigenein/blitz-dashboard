//! CLI options.

use std::str::FromStr;
use std::time::Duration as StdDuration;

use anyhow::anyhow;
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
        parse(from_occurrences = parse_verbosity),
    )]
    pub verbosity: LevelFilter,

    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

fn parse_verbosity(n_occurences: u64) -> LevelFilter {
    match n_occurences {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
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

fn parse_task_count(value: &str) -> crate::Result<usize> {
    match usize::from_str(value)? {
        count if count >= 1 => Ok(count),
        _ => Err(anyhow!("expected non-zero number of tasks")),
    }
}

/// Runs the account crawler
#[derive(StructOpt)]
pub struct CrawlerOpts {
    #[structopt(flatten)]
    pub connections: ConnectionOpts,

    /// Minimum last battle time offset for the «slow» sub-crawler
    #[structopt(long, default_value = "1w", parse(try_from_str = humantime::parse_duration))]
    pub slow_offset: StdDuration,

    /// Minimum last battle time offset for the «fast» sub-crawler
    #[structopt(long, default_value = "5m", parse(try_from_str = humantime::parse_duration))]
    pub min_offset: StdDuration,

    /// Number of tasks for the «fast» sub-crawler
    #[structopt(
        long,
        default_value = "1",
        parse(try_from_str = parse_task_count),
    )]
    pub n_fast_tasks: usize,

    /// Metrics logging interval
    #[structopt(long, default_value = "1m", parse(try_from_str = humantime::parse_duration))]
    pub log_interval: StdDuration,

    /// Maximum training stream size
    #[structopt(
        long,
        default_value = "7500000",
        parse(try_from_str = parse_non_zero_usize),
    )]
    pub training_stream_size: usize,
}

fn parse_non_zero_usize(value: &str) -> crate::Result<usize> {
    match FromStr::from_str(value)? {
        limit if limit >= 1 => Ok(limit),
        _ => Err(anyhow!("expected a positive size")),
    }
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
    pub connections: ConnectionOpts,

    /// Starting account ID
    #[structopt(long, parse(try_from_str = parse_account_id))]
    pub start_id: i32,

    /// Ending account ID (non-inclusive)
    #[structopt(long, parse(try_from_str = parse_account_id))]
    pub end_id: i32,

    /// Number of tasks for the crawler
    #[structopt(
        long,
        default_value = "1",
        parse(try_from_str = parse_task_count),
    )]
    pub n_tasks: usize,
}

fn parse_account_id(value: &str) -> crate::Result<i32> {
    match i32::from_str(value)? {
        account_id if account_id >= 1 => Ok(account_id),
        account_id => Err(anyhow!("{} is an invalid account ID", account_id)),
    }
}

/// Trains the collaborative filtering model
#[derive(StructOpt)]
pub struct TrainerOpts {
    /// Redis URI
    #[structopt(long, default_value = "redis://127.0.0.1/0")]
    pub redis_uri: String,

    /// Account latent vector learning rate
    #[structopt(long = "account-lr", default_value = "0.001")]
    pub account_learning_rate: f64,

    /// Vehicle latent vector learning rate
    #[structopt(long = "vehicle-lr", default_value = "0.001")]
    pub vehicle_learning_rate: f64,

    /// Regularization
    #[structopt(short = "r", long = "regularization", default_value = "0.000001")]
    pub regularization: f64,

    /// Number of latent factors
    #[structopt(short = "f", long = "factors", default_value = "8")]
    pub n_factors: usize,

    /// Standard deviation of newly initialised latent factors
    #[structopt(long, default_value = "0.1")]
    pub factor_std: f64,

    /// Maximum account idle time after which the account factors expire
    #[structopt(long, default_value = "2months", parse(try_from_str = humantime::parse_duration))]
    pub account_ttl: StdDuration,

    /// Approximate time span of the training set (most recent battles)
    #[structopt(long, default_value = "2days", parse(try_from_str = humantime::parse_duration))]
    pub time_span: StdDuration,

    /// Number of cached account latent vectors – must be at least the number of unique accounts in the training set
    #[structopt(
        long,
        default_value = "1000000",
        parse(try_from_str = parse_non_zero_usize),
    )]
    pub account_cache_size: usize,
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
