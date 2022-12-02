//! CLI options.

use std::num::NonZeroU32;
use std::path::PathBuf;

use clap::builder::EnumValueParser;
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
        default_value = "0.01",
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
    TestReplay(TestReplayOpts),
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

    /// Google Analytics measurement ID.
    #[structopt(long, env = "BLITZ_DASHBOARD_WEB_GTAG")]
    pub gtag: Option<String>,

    #[structopt(
        long,
        env = "BLITZ_DASHBOARD_WEB_TRAINER_BASE_URL",
        default_value = "http://localhost:8082"
    )]
    pub trainer_base_url: String,
}

/// Runs the account crawler.
#[derive(Parser)]
pub struct CrawlerOpts {
    #[clap(flatten)]
    pub shared: SharedCrawlerOpts,

    /// Minimum last battle time offset.
    #[clap(
        long,
        default_value = "0s",
        value_parser = humantime::parse_duration,
        env = "BLITZ_DASHBOARD_CRAWLER_MIN_OFFSET",
    )]
    pub min_offset: time::Duration,

    #[clap(
        long,
        default_value = "24h",
        value_parser = humantime::parse_duration,
        env = "BLITZ_DASHBOARD_CRAWLER_OFFSET_SCALE",
    )]
    pub offset_scale: time::Duration,

    /// Number of accounts to sample from the database in one query.
    #[clap(
        long,
        default_value = "100",
        value_parser = parsers::non_zero_usize,
        env = "BLITZ_DASHBOARD_CRAWLER_SAMPLE_SIZE",
    )]
    pub sample_size: usize,

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
    #[clap(long, value_parser = parsers::account_id)]
    pub start_id: wargaming::AccountId,

    /// Ending account ID (non-inclusive).
    #[clap(long, value_parser = parsers::account_id)]
    pub end_id: wargaming::AccountId,
}

#[derive(Parser)]
pub struct BufferingOpts {
    /// Number of account batches which should get concurrently crawled.
    /// A batch is 100 accounts – the maximum for Wargaming.net API.
    /// Note, that buffered batches are not necessarily full – they may even be empty
    /// (if no account in the batch has played since the last update).
    /// Use this to adjust the API load (requests per second).
    #[structopt(
        long = "n-buffered-batches",
        default_value = "1",
        value_parser = parsers::non_zero_usize,
        env = "BLITZ_DASHBOARD_CRAWLER_BUFFERED_BATCHES",
    )]
    pub n_batches: usize,
}

#[derive(Parser)]
pub struct SharedCrawlerOpts {
    #[clap(flatten)]
    pub connections: ConnectionOpts,

    /// Specifies which realm should be crawled.
    #[clap(
        long,
        ignore_case = true,
        value_parser = EnumValueParser::<wargaming::Realm>::new(),
        env = "BLITZ_DASHBOARD_CRAWLER_REALM",
    )]
    pub realm: wargaming::Realm,

    #[clap(flatten)]
    pub buffering: BufferingOpts,

    /// Metrics logging interval.
    #[structopt(
        long,
        default_value = "1min",
        value_parser = humantime::parse_duration,
        env = "BLITZ_DASHBOARD_CRAWLER_LOG_INTERVAL",
    )]
    pub log_interval: time::Duration,
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
        value_parser = humantime::parse_duration,
        env = "BLITZ_DASHBOARD_API_TIMEOUT",
    )]
    pub api_timeout: time::Duration,

    /// Maximum number of requests per second for the API.
    #[clap(long, env = "BLITZ_DASHBOARD_MAX_API_RPS", default_value = "19")]
    pub max_api_rps: NonZeroU32,
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
        default_value = "mongodb://localhost/yastatist?directConnection=true",
        env = "BLITZ_DASHBOARD_MONGODB_URI"
    )]
    pub mongodb_uri: String,
}

/// Parses a replay file.
#[derive(Parser)]
pub struct TestReplayOpts {
    #[structopt(value_name = "REPLAY_PATH")]
    pub path: PathBuf,
}
