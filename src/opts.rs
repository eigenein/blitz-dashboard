//! CLI options.

mod parsers;

use chrono::Duration;
use std::time::Duration as StdDuration;

use log::LevelFilter;
use rust_decimal::Decimal;
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
        parse(try_from_str = parsers::task_count),
    )]
    pub n_fast_tasks: usize,

    /// Metrics logging interval
    #[structopt(long, default_value = "1m", parse(try_from_str = humantime::parse_duration))]
    pub log_interval: StdDuration,

    /// Maximum training stream size
    #[structopt(
        long,
        default_value = "7500000",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub training_stream_size: usize,
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
    #[structopt(long, parse(try_from_str = parsers::account_id))]
    pub start_id: i32,

    /// Ending account ID (non-inclusive)
    #[structopt(long, parse(try_from_str = parsers::account_id))]
    pub end_id: i32,

    /// Number of tasks for the crawler
    #[structopt(
        long,
        default_value = "1",
        parse(try_from_str = parsers::task_count),
    )]
    pub n_tasks: usize,
}

/// Trains the collaborative filtering model
#[derive(StructOpt)]
pub struct TrainerOpts {
    /// Redis URI
    #[structopt(long, default_value = "redis://127.0.0.1/0")]
    pub redis_uri: String,

    /// Learning rate
    #[structopt(long = "lr", default_value = "0.2")]
    pub learning_rate: f64,

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
    #[structopt(long = "account-ttl", default_value = "2months", parse(try_from_str = parsers::duration_as_secs))]
    pub account_ttl_secs: usize,

    /// Time span of the training set (most recent battles)
    #[structopt(long, default_value = "2days", parse(try_from_str = parsers::duration))]
    pub time_span: Duration,

    /// Maximum number of cached account latent vectors
    #[structopt(
        long,
        default_value = "500000",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub account_cache_size: usize,

    /// Run the grid search, perform the specified number of epochs for each set of parameters
    #[structopt(long = "gse")]
    pub n_grid_search_epochs: Option<usize>,

    #[structopt(long = "gsr")]
    pub grid_search_regularizations: Vec<Decimal>,

    #[structopt(long = "gsf")]
    pub grid_search_factors: Vec<usize>,

    #[structopt(long = "gsi", default_value = "3")]
    pub grid_search_iterations: usize,
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
