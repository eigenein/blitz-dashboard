use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::FutureExt;
use log::Level;
use sentry::integrations::anyhow::capture_anyhow;
use sqlx::{PgConnection, PgPool};

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BaseAccountInfo, Tank};
use crate::opts::CrawlerOpts;
use crate::wargaming::WargamingApi;
use rocket::route::BoxFuture;

pub async fn run(api: WargamingApi, opts: CrawlerOpts) -> crate::Result {
    Crawler {
        api,
        database: crate::database::open(&opts.database).await?,
        once: opts.once,
        n_chunks: opts.n_chunks,
    }
    .run()
    .await
}

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    once: bool,
    n_chunks: i32,
}

#[derive(Debug, PartialEq)]
enum CrawlMode {
    New,
    LastBattleTime(DateTime<Utc>),
}

impl Crawler {
    pub async fn run(&self) -> crate::Result {
        sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

        loop {
            let stopwatch = Stopwatch::new("Iteration finished").level(Level::Info);

            let mut batch =
                retrieve_batch(&self.database, 50 * self.n_chunks, 50 * self.n_chunks).await?;
            fastrand::shuffle(&mut batch);

            let results = futures::future::join_all(
                batch
                    .chunks(100)
                    .into_iter()
                    .enumerate()
                    .map(|(i, chunk)| self.crawl_chunk(i, chunk).boxed())
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
                stopwatch.elapsed().as_secs_f64() / self.n_chunks as f64,
            );

            if self.once {
                break Ok(());
            }
        }
    }

    async fn crawl_chunk(
        &self,
        chunk_index: usize,
        previous_infos: &[BaseAccountInfo],
    ) -> crate::Result {
        let _stopwatch = Stopwatch::new(format!(
            "Chunk #{} ({} accounts) crawled",
            chunk_index,
            previous_infos.len(),
        ))
        .level(Level::Info);

        let account_ids = previous_infos
            .iter()
            .map(|account| account.id)
            .collect::<Vec<_>>();
        let mut current_infos = self.api.get_account_info(&account_ids).await?;

        let mut transaction = self.database.begin().await?;

        for previous_info in previous_infos.iter() {
            let current_info = current_infos
                .remove(&previous_info.id.to_string())
                .flatten();
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

        log::debug!("Committing…");
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
                log::warn!("The account does not exist anymore. Deleting…");
                database::delete_account(&self.database, account_id).await?;
            }
            _ => {}
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
            -- FIXME: there are some pitfalls.
            SELECT * FROM accounts
            WHERE last_battle_time < NOW() - INTERVAL '1 minute'
            ORDER BY last_battle_time DESC
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
