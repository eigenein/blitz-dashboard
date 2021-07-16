use clap::{crate_authors, crate_description, crate_name, crate_version, AppSettings, Clap};

#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = crate_description!())]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(short, long, about = "Wargaming.net API application ID")]
    pub application_id: String,

    #[clap(short, long, about = "Sentry DSN")]
    pub sentry_dsn: Option<String>,

    #[clap(short, long, about = "PostgreSQL database URI")]
    pub database: String,

    #[clap(
        short = 'v',
        long = "verbose",
        about = "Increases log verbosity",
        parse(from_occurrences)
    )]
    pub verbosity: i32,

    #[clap(long, about = "Runs the web app")]
    pub web: bool,

    #[clap(long, about = "Runs the account crawler")]
    pub crawler: bool,

    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Clap)]
pub enum Subcommand {
    Web(WebOpts),
    Crawler(CrawlerOpts),
}

#[derive(Clap)]
#[clap(name = crate_name!())]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = "Runs the web application")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct WebOpts {
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
pub struct CrawlerOpts;

pub fn parse() -> Opts {
    Opts::parse()
}
