use clap::{crate_authors, crate_description, crate_name, crate_version, AppSettings, Clap};

#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = crate_description!())]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(short, long, about = "Sentry DSN")]
    pub sentry_dsn: Option<String>,

    #[clap(
        short = 'v',
        long = "verbose",
        about = "Increases log verbosity",
        parse(from_occurrences)
    )]
    pub verbosity: i32,

    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Clap)]
pub enum Subcommand {
    Web(WebOpts),
    Crawler(CrawlerOpts),
    ImportTankopedia(ImportTankopediaOpts),
    CrawlAccounts(CrawlAccountsOpts),
}

#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = "Runs the web application")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct WebOpts {
    #[clap(short, long, about = "PostgreSQL database URI")]
    pub database: String,

    #[clap(
        short,
        long,
        about = "Wargaming.net API application ID",
        env = "BLITZ_DASHBOARD_APPLICATION_ID"
    )]
    pub application_id: String,

    #[clap(long, default_value = "::", about = "Web app host")]
    pub host: String,

    #[clap(short, long, default_value = "8081", about = "Web app port")]
    pub port: u16,

    #[clap(long, about = "Yandex.Metrika counter number")]
    pub yandex_metrika: Option<String>,

    #[clap(long, about = "Google Analytics measurement ID")]
    pub gtag: Option<String>,
}

#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = "Runs the account crawler")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct CrawlerOpts {
    #[clap(short, long, about = "PostgreSQL database URI")]
    pub database: String,

    #[clap(
        short,
        long,
        about = "Wargaming.net API application ID",
        env = "BLITZ_DASHBOARD_APPLICATION_ID"
    )]
    pub application_id: String,

    #[clap(short, long, about = "Number of crawling tasks")]
    pub n_tasks: usize,
}

#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = "Updates the bundled Tankopedia module")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct ImportTankopediaOpts {
    #[clap(
        short,
        long,
        about = "Wargaming.net API application ID",
        env = "BLITZ_DASHBOARD_APPLICATION_ID"
    )]
    pub application_id: String,
}

// TODO: add batch size.
#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = "Crawls the specified account IDs")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct CrawlAccountsOpts {
    #[clap(short, long, about = "PostgreSQL database URI")]
    pub database: String,

    #[clap(
        short,
        long,
        about = "Wargaming.net API application ID",
        env = "BLITZ_DASHBOARD_APPLICATION_ID"
    )]
    pub application_id: String,

    #[clap(long, about = "Starting account ID")]
    pub start_id: i32,

    #[clap(long, about = "Ending account ID (non-inclusive)")]
    pub end_id: i32,

    #[clap(short, long, about = "Number of crawling tasks")]
    pub n_tasks: usize,
}
