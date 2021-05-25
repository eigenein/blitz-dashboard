use crate::api::wargaming::WargamingApi;
use crate::opts::{Opts, Subcommand};
use clap::{crate_name, crate_version};
use mongodb::Database;
use sentry::integrations::anyhow::capture_anyhow;

mod api;
mod logging;
mod opts;
mod web;

type Result<T = ()> = anyhow::Result<T>;

#[async_std::main]
async fn main() -> crate::Result {
    let opts = opts::parse();
    logging::init(opts.debug)?;
    log::info!("{} {}", crate_name!(), crate_version!());
    let _sentry_guard = init_sentry(&opts);
    let result = run_subcommand(opts).await;
    if let Err(ref error) = result {
        capture_anyhow(error);
    }
    result
}

async fn run_subcommand(opts: Opts) -> crate::Result {
    let database = init_database(&opts.mongodb_uri).await?;
    match opts.subcommand {
        Subcommand::Web(web_opts) => {
            web::run(
                &web_opts.host,
                web_opts.port,
                WargamingApi::new(&opts.application_id),
                database,
            )
            .await
        }
        Subcommand::Crawler(_) => unimplemented!(),
    }
}

/// Initialize Sentry.
/// See also: <https://docs.sentry.io/platforms/rust/>.
fn init_sentry(opts: &opts::Opts) -> Option<sentry::ClientInitGuard> {
    opts.sentry_dsn.as_ref().map(|dsn| {
        sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                debug: opts.debug,
                ..Default::default()
            },
        ))
    })
}

async fn init_database(uri: &str) -> crate::Result<Database> {
    let client = mongodb::Client::with_uri_str(uri).await?;
    let database = client.database("blitz-dashboard");
    database
        .run_command(
            mongodb::bson::doc! {
                "createIndexes": "accounts",
                "indexes": [{
                    "key": {"account_id": 1, "last_battle_time": -1},
                    "name": "account_id_last_battle_time",
                    "unique": true,
                }],
            },
            None,
        )
        .await?;
    Ok(database)
}
