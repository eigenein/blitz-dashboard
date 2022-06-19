//! CLI options.

use clap::Parser;

use crate::prelude::*;
use crate::wargaming;

mod parsers;

#[derive(Parser)]
#[clap(author, version, about, rename_all = "kebab-case")]
pub struct Opts {
    /// Sentry DSN.
    #[clap(long, env = "BLITZ_DASHBOARD_SENTRY_DSN")]
    pub sentry_dsn: Option<String>,

    /// Performance traces sample rate for Sentry.
    #[clap(
        long,
        default_value = "0.1",
        env = "BLITZ_DASHBOARD_TRACES_SAMPLE_RATE"
    )]
    pub traces_sample_rate: f32,

    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
    Crawl(CrawlerOpts),
    CrawlAccounts(CrawlAccountsOpts),
    ImportTankopedia(ImportTankopediaOpts),
    Web(WebOpts),
}

/// Runs the web application.
#[derive(Parser)]
pub struct WebOpts {
    #[clap(flatten)]
    pub connections: ConnectionOpts,

    /// Web application bind host.
    #[clap(long, default_value = "::", env = "BLITZ_DASHBOARD_WEB_BIND_HOST")]
    pub host: String,

    /// Web application bind port.
    #[structopt(long, default_value = "8081", env = "BLITZ_DASHBOARD_WEB_BIND_PORT")]
    pub port: u16,

    /// Yandex.Metrika counter number (deprecated).
    #[structopt(long)]
    pub yandex_metrika: Option<String>,

    /// Google Analytics measurement ID.
    #[structopt(long, env = "BLITZ_DASHBOARD_WEB_GTAG")]
    pub gtag: Option<String>,
}

/// Runs the account crawler.
#[derive(Parser)]
pub struct CrawlerOpts {
    #[clap(flatten)]
    pub shared: SharedCrawlerOpts,

    /// Minimum last battle time offset.
    #[clap(
        long,
        default_value = "8hours",
        parse(try_from_str = humantime::parse_duration),
        env = "BLITZ_DASHBOARD_CRAWLER_MIN_OFFSET",
    )]
    pub min_offset: StdDuration,

    /// Maximum last battle time offset.
    #[clap(
        long,
        default_value = "3years",
        parse(try_from_str = humantime::parse_duration),
        env = "BLITZ_DASHBOARD_CRAWLER_MAX_OFFSET",
    )]
    pub max_offset: StdDuration,

    /// Number of accounts to sample from the database in one query.
    #[clap(
        long,
        default_value = "100",
        parse(try_from_str = parsers::non_zero_u32),
        env = "BLITZ_DASHBOARD_CRAWLER_SAMPLE_SIZE",
    )]
    pub sample_size: u32,

    #[clap(long, env = "BLITZ_DASHBOARD_CRAWLER_HEARTBEAT_URL")]
    pub heartbeat_url: Option<String>,
}

/// Updates the bundled Tankopedia module.
#[derive(Parser)]
pub struct ImportTankopediaOpts {
    /// Wargaming.net API application ID.
    #[structopt(short, long, env = "BLITZ_DASHBOARD_APPLICATION_ID")]
    pub application_id: String,
}

/// Crawls the specified account IDs.
#[derive(Parser)]
pub struct CrawlAccountsOpts {
    #[clap(flatten)]
    pub shared: SharedCrawlerOpts,

    /// Starting account ID.
    #[clap(long, parse(try_from_str = parsers::account_id))]
    pub start_id: wargaming::AccountId,

    /// Ending account ID (non-inclusive).
    #[clap(long, parse(try_from_str = parsers::account_id))]
    pub end_id: wargaming::AccountId,
}

#[derive(Parser)]
pub struct BufferingOpts {
    /// Number of account batches which should get concurrently crawled and buffered.
    /// A batch is 100 accounts – the maximum for Wargaming.net API.
    /// Each batch needs one API call (basic account information).
    /// A buffered batch contains accounts which last battle timestamp has changed,
    /// they are ready to be crawled.
    /// Note, that buffered batches are not necessarily full – they may even be empty
    /// (if no account in the batch has played since the last update).
    /// Use this to adjust the API load (requests per second).
    #[structopt(
        long = "n-buffered-batches",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_BUFFERED_BATCHES",
    )]
    pub n_batches: usize,

    /// Number of accounts being concurrently crawled.
    /// Each account needs 2 API calls (tanks statistics and achievements).
    /// Buffered account contains all the information needed to update it in the database.
    /// Use this to adjust the API load (requests per second).
    #[structopt(
        long = "n-buffered-accounts",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_BUFFERED_ACCOUNTS",
    )]
    pub n_buffered_accounts: usize,

    /// Number of already crawled accounts being concurrently updated in the database.
    /// This configures the last step in the crawling pipeline – use it to adjust the database load.
    #[structopt(
        long = "n-updated-accounts",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_UPDATED_ACCOUNTS",
    )]
    pub n_updated_accounts: usize,
}

#[derive(Parser)]
pub struct SharedCrawlerOpts {
    #[clap(flatten)]
    pub connections: ConnectionOpts,

    #[clap(flatten)]
    pub buffering: BufferingOpts,

    /// Metrics logging interval.
    #[structopt(
        long,
        default_value = "1min",
        parse(try_from_str = humantime::parse_duration),
        env = "BLITZ_DASHBOARD_CRAWLER_LOG_INTERVAL",
    )]
    pub log_interval: StdDuration,

    #[structopt(
        long,
        default_value = "95",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_LAG_PERCENTILE",
    )]
    pub lag_percentile: usize,

    #[structopt(
        long,
        default_value = "5000",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_LAG_WINDOW_SIZE",
    )]
    pub lag_window_size: usize,
}

#[derive(Parser)]
pub struct ConnectionOpts {
    #[clap(flatten)]
    pub internal: InternalConnectionOpts,

    /// Wargaming.net API application ID.
    #[clap(short, long, env = "BLITZ_DASHBOARD_APPLICATION_ID")]
    pub application_id: String,

    /// Wargaming.net API timeout.
    #[structopt(
        long,
        default_value = "10sec",
        parse(try_from_str = humantime::parse_duration),
        env = "BLITZ_DASHBOARD_API_TIMEOUT",
    )]
    pub api_timeout: StdDuration,

    /// Maximum number of requests per second for the API.
    #[clap(long, env = "BLITZ_DASHBOARD_MAX_API_RPS", default_value = "20")]
    pub max_api_rps: u64,
}

#[derive(Parser)]
pub struct InternalConnectionOpts {
    /// Redis URI
    #[structopt(
        long,
        default_value = "redis://127.0.0.1/0",
        env = "BLITZ_DASHBOARD_REDIS_URI"
    )]
    pub redis_uri: String,

    /// Redis connection pool size
    #[structopt(long, default_value = "5", env = "BLITZ_DASHBOARD_REDIS_POOL_SIZE")]
    pub redis_pool_size: usize,

    /// MongoDB connection URI
    #[structopt(
        long = "mongodb-uri",
        default_value = "mongodb://localhost/yastatist?compressors=zstd",
        env = "BLITZ_DASHBOARD_MONGODB_URI"
    )]
    pub mongodb_uri: String,
}
