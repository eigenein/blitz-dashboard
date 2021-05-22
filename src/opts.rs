use clap::{crate_authors, crate_description, crate_version, AppSettings, Clap};

#[derive(Clap)]
#[clap(author = crate_authors!())]
#[clap(version = crate_version!())]
#[clap(about = crate_description!())]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(long, default_value = "localhost", about = "Web app host")]
    pub host: String,

    #[clap(short, long, default_value = "8080", about = "Web app port")]
    pub port: u16,

    #[clap(short, long, about = "Wargaming.net API application ID")]
    pub application_id: String,

    #[clap(long, about = "Sentry DSN")]
    pub sentry_dsn: Option<String>,

    #[clap(short, long, about = "Enable debugging")]
    pub debug: bool,
}

pub fn parse() -> Opts {
    Opts::parse()
}
