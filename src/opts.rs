//! CLI options.

use std::str::FromStr;
use std::time::Duration as StdDuration;

use anyhow::anyhow;
use log::LevelFilter;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
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
    Crawler(CrawlerOpts),
    ImportTankopedia(ImportTankopediaOpts),
    CrawlAccounts(CrawlAccountsOpts),
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
}

/// Runs the account crawler
#[derive(StructOpt)]
pub struct CrawlerOpts {
    #[structopt(flatten)]
    pub connections: ConnectionOpts,

    #[structopt(flatten)]
    pub cf: CfOpts,

    /// Time offsets between different sub-crawlers
    #[structopt(short, long = "offset", parse(try_from_str = humantime::parse_duration))]
    pub offsets: Vec<StdDuration>,

    /// Minimum last battle time offset – avoids selecting the same account too soon
    #[structopt(long = "min-offset", default_value = "5m", parse(try_from_str = humantime::parse_duration))]
    pub min_offset: StdDuration,
}

fn parse_task_count(value: &str) -> crate::Result<usize> {
    let value = usize::from_str(value)?;
    if value != 0 {
        Ok(value)
    } else {
        Err(anyhow!("expected non-zero number of tasks"))
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

    #[structopt(flatten)]
    pub cf: CfOpts,

    /// Starting account ID
    #[structopt(long, parse(try_from_str = parse_account_id))]
    pub start_id: i32,

    /// Ending account ID (non-inclusive)
    #[structopt(long, parse(try_from_str = parse_account_id))]
    pub end_id: i32,

    /// Number of tasks
    #[structopt(
        long,
        default_value = "1",
        parse(try_from_str = parse_task_count),
    )]
    pub n_tasks: usize,
}

fn parse_account_id(value: &str) -> crate::Result<i32> {
    let account_id = i32::from_str(value)?;
    if account_id >= 1 {
        Ok(account_id)
    } else {
        Err(anyhow!("{} is an invalid account ID", account_id))
    }
}

#[derive(StructOpt)]
pub struct ConnectionOpts {
    /// PostgreSQL database URI
    #[structopt(short, long = "database")]
    pub database_uri: String,

    /// Initialize the database schema
    #[structopt(long)]
    pub initialize_schema: bool,

    /// Wargaming.net API application ID
    #[structopt(short, long)]
    pub application_id: String,

    /// Redis URI
    #[structopt(short, long, default_value = "redis://127.0.0.1/0")]
    pub redis_uri: String,
}

#[derive(StructOpt, Clone)]
pub struct CfOpts {
    /// Account factor learning rate for the win rate prediction.
    #[structopt(long = "account-lr", default_value = "0.1")]
    pub account_learning_rate: f64,

    /// Vehicle factor learning rate for the win rate prediction.
    #[structopt(long = "vehicle-lr", default_value = "0.01")]
    pub vehicle_learning_rate: f64,

    /// Global bias learning rate for the win rate prediction.
    #[structopt(long = "--global-bias-lr", default_value = "0.00001")]
    pub global_bias_learning_rate: f64,
}
