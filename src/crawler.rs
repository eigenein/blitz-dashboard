use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};
use futures::FutureExt;
use itertools::Itertools;
use log::Level;
use rocket::route::BoxFuture;
use sentry::integrations::anyhow::capture_anyhow;
use sqlx::{PgConnection, PgPool};

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BaseAccountInfo, Tank};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::wargaming::WargamingApi;

pub async fn run(opts: CrawlerOpts) -> crate::Result {
    Crawler::new(&opts.application_id, &opts.database)
        .await?
        .run(opts.n_chunks, opts.once)
        .await
}

pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    let crawler = Crawler::new(&opts.application_id, &opts.database).await?;
    let epoch = Utc.timestamp(0, 0);
    for chunk in &(opts.start_id..opts.end_id).chunks(100) {
        let previous_infos: Vec<BaseAccountInfo> = chunk
            .map(|account_id| BaseAccountInfo {
                id: account_id,
                crawled_at: None,
                // FIXME: the fields below don't matter for `crawl_chunk`, but would be better without the hack.
                last_battle_time: epoch,
                nickname: "".to_string(),
                created_at: epoch,
            })
            .collect();
        crawler.crawl_chunk(&previous_infos).await?;
    }
    Ok(())
}

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
}

#[derive(Debug, PartialEq)]
enum CrawlMode {
    New,
    LastBattleTime(DateTime<Utc>),
}

impl Crawler {
    pub async fn new(application_id: &str, database_uri: &str) -> crate::Result<Self> {
        Ok(Self {
            api: WargamingApi::new(application_id, StdDuration::from_millis(1500))?,
            database: crate::database::open(database_uri).await?,
        })
    }

    pub async fn run(&self, n_chunks: i32, once: bool) -> crate::Result {
        sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

        loop {
            let stopwatch = Stopwatch::new("Iteration finished").level(Level::Info);

            // FIXME: read only `account_id`, `crawled_at` and `last_battle_time`.
            let mut batch = retrieve_batch(&self.database, 50 * n_chunks, 50 * n_chunks).await?;
            fastrand::shuffle(&mut batch);

            let results = futures::future::join_all(
                batch
                    .chunks(100)
                    .into_iter()
                    .map(|chunk| self.crawl_chunk(chunk).boxed())
                    .collect::<Vec<BoxFuture<crate::Result>>>(),
            )
            .await;

            for result in results {
                if let Err(error) = result {
                    let sentry_id = capture_anyhow(&error);
                    log::error!("Failed to crawl the chunk: {} (https://sentry.io/eigenein/blitz-dashboard/events/{})", error, sentry_id);
                }
            }

            log::info!(
                "{:.3}s per chunk.",
                stopwatch.elapsed().as_secs_f64() / n_chunks as f64,
            );

            if once {
                break Ok(());
            }
        }
    }

    async fn crawl_chunk(&self, previous_infos: &[BaseAccountInfo]) -> crate::Result {
        let _stopwatch =
            Stopwatch::new(format!("Chunk ({} accounts) crawled", previous_infos.len()))
                .level(Level::Info);

        let account_ids = previous_infos
            .iter()
            .map(|account| account.id)
            .collect::<Vec<_>>();
        let mut current_infos = self.api.get_account_info(&account_ids).await?;

        let mut transaction = self.database.begin().await?;

        let mut n_accounts = 0;
        for previous_info in previous_infos.iter() {
            let current_info = current_infos
                .remove(&previous_info.id.to_string())
                .flatten();
            if current_info.is_some() {
                n_accounts += 1;
            }
            self.crawl_account(
                &mut transaction,
                previous_info.id,
                match previous_info.crawled_at {
                    Some(_) => CrawlMode::LastBattleTime(previous_info.last_battle_time),
                    None => CrawlMode::New,
                },
                current_info,
            )
            .await?;
        }

        log::info!("{} accounts found. Committing…", n_accounts);
        transaction.commit().await?;

        Ok(())
    }

    async fn crawl_account(
        &self,
        connection: &mut PgConnection,
        account_id: i32,
        mode: CrawlMode,
        current_info: Option<AccountInfo>,
    ) -> crate::Result {
        let _stopwatch =
            Stopwatch::new(format!("Account #{} crawled", account_id)).level(Level::Info);
        log::info!("Crawling account #{}, {:?}…", account_id, mode);

        match current_info {
            Some(current_info) => {
                self.crawl_existing_account(&mut *connection, account_id, mode, current_info)
                    .await?;
            }
            None if mode != CrawlMode::New => {
                log::warn!("The account #{} does not exist. Deleting…", account_id);
                database::delete_account(&self.database, account_id).await?;
            }
            _ => {
                log::info!("The account #{} does not exist.", account_id);
            }
        };

        Ok(())
    }

    async fn crawl_existing_account(
        &self,
        connection: &mut PgConnection,
        account_id: i32,
        mode: CrawlMode,
        mut current_info: AccountInfo,
    ) -> crate::Result {
        log::debug!("Nickname: {}.", current_info.base.nickname);
        current_info.base.crawled_at = Some(Utc::now());
        database::insert_account_or_replace(&mut *connection, &current_info.base).await?;

        match mode {
            CrawlMode::New => {
                database::insert_account_snapshot(&mut *connection, &current_info).await?;
                let tanks = self.api.get_merged_tanks(account_id).await?;
                database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            }
            CrawlMode::LastBattleTime(last_battle_time)
                if current_info.base.last_battle_time != last_battle_time =>
            {
                database::insert_account_snapshot(&mut *connection, &current_info).await?;
                let tanks: Vec<Tank> = self
                    .api
                    .get_merged_tanks(account_id)
                    .await?
                    .into_iter()
                    .filter(|tank| tank.last_battle_time > last_battle_time)
                    .collect();
                database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            }
            _ => log::info!("No new battles detected."),
        };

        Ok(())
    }
}

async fn retrieve_batch(
    connection: &PgPool,
    n_least_recently_crawled: i32,
    n_most_recently_played: i32,
) -> crate::Result<Vec<BaseAccountInfo>> {
    // language=SQL
    const QUERY: &str = r#"
        (SELECT * FROM accounts ORDER BY crawled_at NULLS FIRST LIMIT $1)
        UNION
        (
            SELECT * FROM accounts
            WHERE last_battle_time > NOW() - INTERVAL '1 hour'
            ORDER BY crawled_at
            LIMIT $2
        );
    "#;
    let accounts = sqlx::query_as(QUERY)
        .bind(n_least_recently_crawled)
        .bind(n_most_recently_played)
        .fetch_all(connection)
        .await
        .context("failed to retrieve a batch")?;
    Ok(accounts)
}
