//! CLI options.

use clap::Parser;

use crate::prelude::*;

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
    Web(WebOpts),

    #[clap(alias = "crawler")]
    Crawl(CrawlerOpts),

    ImportTankopedia(ImportTankopediaOpts),

    CrawlAccounts(CrawlAccountsOpts),

    InitializeDatabase(InitializeDatabaseOpts),
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
        default_value = "12hours",
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

    /// Limit for the inner query when retrieving a batch from the database.
    /// Lower is faster, larger keeps the mean batch size closer to the maximum (which is better).
    /// Aim for `100` in the `BS` metric. See also `crate::crawler::batch_stream::retrieve_batch`.
    #[clap(
        long,
        default_value = "1000",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_BATCH_SELECT_LIMIT",
    )]
    pub batch_select_limit: usize,
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
    pub start_id: i32,

    /// Ending account ID (non-inclusive).
    #[clap(long, parse(try_from_str = parsers::account_id))]
    pub end_id: i32,
}

#[derive(Parser)]
pub struct BufferingOpts {
    /// Number of buffered batches in the stream.
    #[structopt(
        long = "n-buffered-batches",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_BUFFERED_BATCHES",
    )]
    pub n_batches: usize,

    /// Number of buffered accounts in the stream.
    #[structopt(
        long = "n-buffered-accounts",
        default_value = "1",
        parse(try_from_str = parsers::non_zero_usize),
        env = "BLITZ_DASHBOARD_CRAWLER_BUFFERED_ACCOUNTS",
    )]
    pub n_accounts: usize,
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
}

/// Initializes the database schema.
#[derive(Clone, Parser)]
pub struct InitializeDatabaseOpts {
    /// PostgreSQL database URI.
    #[clap(
        long = "postgres-uri",
        default_value = "postgres://localhost/yastatist",
        env = "BLITZ_DASHBOARD_POSTGRES_URI"
    )]
    pub database_uri: String,
}

#[derive(Parser)]
pub struct ConnectionOpts {
    #[clap(flatten)]
    pub internal: InternalConnectionOpts,

    /// Wargaming.net API application ID.
    #[clap(short, long, env = "BLITZ_DASHBOARD_APPLICATION_ID")]
    pub application_id: String,
}

#[derive(Parser)]
pub struct InternalConnectionOpts {
    /// PostgreSQL database URI.
    #[clap(
        long = "postgres-uri",
        default_value = "postgres://localhost/yastatist",
        env = "BLITZ_DASHBOARD_POSTGRES_URI"
    )]
    pub database_uri: String,

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
