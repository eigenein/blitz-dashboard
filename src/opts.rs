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
    pub connections: ConnectionOpts,

    /// Minimum last battle time offset
    #[structopt(long, default_value = "0s", parse(try_from_str = humantime::parse_duration))]
    pub min_offset: StdDuration,

    /// Turn on the automatic minimum last battle offset adjustment based on 50%-lag (experimental)
    #[structopt(long)]
    pub auto_min_offset: bool,

    /// Number of buffered accounts used to parallelize the crawling process
    #[structopt(
        long,
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub n_buffered_accounts: usize,

    /// Metrics logging interval. With `--auto-min-offset` – also the minimum offset update interval
    #[structopt(long, default_value = "1min", parse(try_from_str = humantime::parse_duration))]
    pub log_interval: StdDuration,

    /// Maximum training stream size
    #[structopt(
        long,
        default_value = "20000000",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub training_stream_size: usize,

    /// Maximum training stream duration
    #[structopt(long, default_value = "7days", parse(try_from_str = parsers::duration))]
    pub training_stream_duration: Duration,

    /// Percentage of points sent to the test sample
    #[structopt(
        long,
        default_value = "5",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub test_percentage: usize,
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

    /// Number of buffered accounts used to parallelize the crawling process
    #[structopt(
        long,
        alias = "n-tasks",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub n_buffered_accounts: usize,

    /// Metrics logging interval
    #[structopt(long, default_value = "30sec", parse(try_from_str = humantime::parse_duration))]
    pub log_interval: StdDuration,
}

/// Trains the collaborative filtering model
#[derive(Clone, StructOpt)]
pub struct TrainerOpts {
    /// Redis URI
    #[structopt(long, default_value = "redis://127.0.0.1/0")]
    pub redis_uri: String,

    /// Log every n-th epoch metrics
    #[structopt(
        long,
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub log_epochs: usize,

    /// Time span of the training set (most recent battles)
    #[structopt(long, default_value = "2days", parse(try_from_str = parsers::duration))]
    pub time_span: Duration,

    #[structopt(flatten)]
    pub model: TrainerModelOpts,

    /// Enable automatic regularization adjustment (experimental)
    #[structopt(long)]
    pub auto_r: bool,

    /// If enabled, unconditionally increases regularization for next epoch by `0.001`
    /// with the specified probability
    #[structopt(long)]
    pub auto_r_bump_chance: Option<f64>,
}

#[derive(Copy, Clone, StructOpt)]
pub struct TrainerModelOpts {
    /// Learning rate
    #[structopt(long = "lr", default_value = "0.001")]
    pub learning_rate: f64,

    /// Number of latent factors.
    /// Ignored for the grid search.
    #[structopt(short = "f", long = "factors", default_value = "8")]
    pub n_factors: usize,

    /// Standard deviation of newly initialised latent factors
    #[structopt(long, default_value = "0.01")]
    pub factor_std: f64,

    /// Maximum number of cached account latent vectors
    #[structopt(
        long,
        default_value = "750000",
        parse(try_from_str = parsers::non_zero_usize),
    )]
    pub account_cache_size: usize,

    /// Store the latent vectors with the specified period
    #[structopt(
        long,
        alias = "flush-period",
        default_value = "1minute",
        parse(try_from_str = humantime::parse_duration),
    )]
    pub flush_interval: StdDuration,
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
