use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Opts {
    #[structopt(short, long, about = "Sentry DSN", env = "BLITZ_DASHBOARD_SENTRY_DSN")]
    pub sentry_dsn: Option<String>,

    #[structopt(
        short = "v",
        long = "verbose",
        about = "Increases log verbosity",
        parse(from_occurrences)
    )]
    pub verbosity: i32,

    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(StructOpt)]
pub enum Subcommand {
    Web(WebOpts),
    Crawler(CrawlerOpts),
    ImportTankopedia(ImportTankopediaOpts),
    CrawlAccounts(CrawlAccountsOpts),
}

#[derive(StructOpt)]
#[structopt(about = "Runs the web application")]
pub struct WebOpts {
    #[structopt(flatten)]
    pub shared: SharedOpts,

    #[structopt(
        long,
        default_value = "::",
        about = "Web app host",
        env = "BLITZ_DASHBOARD_WEB_HOST"
    )]
    pub host: String,

    #[structopt(
        short,
        long,
        default_value = "8081",
        about = "Web app port",
        env = "BLITZ_DASHBOARD_WEB_PORT"
    )]
    pub port: u16,

    #[structopt(
        long,
        about = "Yandex.Metrika counter number",
        env = "BLITZ_DASHBOARD_WEB_YANDEX_METRIKA"
    )]
    pub yandex_metrika: Option<String>,

    #[structopt(
        long,
        about = "Google Analytics measurement ID",
        env = "BLITZ_DASHBOARD_WEB_GTAG"
    )]
    pub gtag: Option<String>,
}

#[derive(StructOpt)]
#[structopt(about = "Runs the account crawler")]
pub struct CrawlerOpts {
    #[structopt(flatten)]
    pub shared: SharedOpts,

    #[structopt(
        short,
        long,
        about = "Number of crawling tasks",
        default_value = "1",
        env = "BLITZ_DASHBOARD_TASK_COUNT"
    )]
    pub n_tasks: usize,
}

#[derive(StructOpt)]
#[structopt(about = "Updates the bundled Tankopedia module")]
pub struct ImportTankopediaOpts {
    #[structopt(
        short,
        long,
        about = "Wargaming.net API application ID",
        env = "BLITZ_DASHBOARD_APPLICATION_ID"
    )]
    pub application_id: String,
}

#[derive(StructOpt)]
#[structopt(about = "Crawls the specified account IDs")]
pub struct CrawlAccountsOpts {
    #[structopt(flatten)]
    pub shared: SharedOpts,

    #[structopt(
        long,
        about = "Starting account ID",
        env = "BLITZ_DASHBOARD_CRAWLER_START_ID"
    )]
    pub start_id: i32,

    #[structopt(
        long,
        about = "Ending account ID (non-inclusive)",
        env = "BLITZ_DASHBOARD_CRAWLER_END_ID"
    )]
    pub end_id: i32,

    #[structopt(
        short,
        long,
        about = "Number of crawling tasks",
        default_value = "1",
        env = "BLITZ_DASHBOARD_TASK_COUNT"
    )]
    pub n_tasks: usize,
}

#[derive(StructOpt)]
pub struct SharedOpts {
    #[structopt(
        short,
        long,
        about = "PostgreSQL database URI",
        env = "BLITZ_DASHBOARD_DATABASE_URI"
    )]
    pub database: String,

    #[structopt(
        short,
        long,
        about = "Wargaming.net API application ID",
        env = "BLITZ_DASHBOARD_APPLICATION_ID"
    )]
    pub application_id: String,
}
