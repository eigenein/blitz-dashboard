use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};
use futures::future::select_all;
use itertools::Itertools;
use smallvec::SmallVec;
use sqlx::{PgConnection, PgPool};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::database;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BaseAccountInfo, Tank};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::wargaming::WargamingApi;

pub async fn run(opts: CrawlerOpts) -> crate::Result {
    Crawler::new(&opts.application_id, &opts.database)
        .await?
        .run(opts.n_tasks)
        .await
}

pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let epoch = Utc.timestamp(0, 0);
    let crawler = Crawler::new(&opts.application_id, &opts.database).await?;

    for chunk in &(opts.start_id..opts.end_id).chunks(100) {
        let previous_infos: Vec<BaseAccountInfo> = chunk
            .map(|account_id| BaseAccountInfo {
                id: account_id,
                crawled_at: epoch,
                last_battle_time: epoch,
                // FIXME: the following fields don't matter for `crawl_chunk`, but would be better without the hack.
                nickname: String::new(),
                created_at: epoch,
            })
            .collect();
        crawler.clone().crawl_batch(previous_infos).await?;
    }
    Ok(())
}

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
}

impl Crawler {
    pub async fn new(application_id: &str, database_uri: &str) -> crate::Result<Self> {
        Ok(Self {
            api: WargamingApi::new(application_id, StdDuration::from_millis(1500))?,
            database: crate::database::open(database_uri).await?,
        })
    }

    /// Runs the crawler indefinitely.
    pub async fn run(&self, n_tasks: usize) -> crate::Result {
        sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

        log::info!("Running crawler…");
        let mut start_instant = Instant::now();
        let mut max_crawled_at = Utc.timestamp(0, 0);
        let mut running_futures: Vec<JoinHandle<crate::Result>> = Vec::new();
        let mut n_crawled_chunks = 0;
        let mut request_counter = self.api.get_request_counter();

        loop {
            if running_futures.len() < n_tasks {
                let batch = self.retrieve_batch(max_crawled_at).await?;
                max_crawled_at = batch.last().expect("expected some accounts").crawled_at;
                running_futures.push(tokio::spawn(self.clone().crawl_batch(batch)));
            } else {
                let (resolved_future, _, remaining_futures) = select_all(running_futures).await;
                resolved_future??;
                running_futures = remaining_futures;
                n_crawled_chunks += 1;
            }
            if start_instant.elapsed().as_secs() > 10 {
                let elapsed = start_instant.elapsed().as_secs_f64();
                log::info!(
                    "{:.2} chunks/second, {:.1} requests/second.",
                    n_crawled_chunks as f64 / elapsed,
                    (self.api.get_request_counter() - request_counter) as f64 / elapsed,
                );
                start_instant = Instant::now();
                n_crawled_chunks = 0;
                request_counter = self.api.get_request_counter();
            }
        }
    }

    async fn crawl_batch(self, previous_infos: Vec<BaseAccountInfo>) -> crate::Result {
        let account_ids: SmallVec<[i32; 128]> =
            previous_infos.iter().map(|account| account.id).collect();
        let mut current_infos = self.api.get_account_info(&account_ids).await?;

        let mut transaction = self.database.begin().await?;
        for previous_info in previous_infos.iter() {
            let current_info = current_infos
                .remove(&previous_info.id.to_string())
                .flatten();
            self.maybe_crawl_account(&mut transaction, previous_info, current_info)
                .await?;
        }
        log::debug!("Committing…");
        transaction.commit().await?;

        Ok(())
    }

    async fn maybe_crawl_account(
        &self,
        connection: &mut PgConnection,
        previous_info: &BaseAccountInfo,
        current_info: Option<AccountInfo>,
    ) -> crate::Result {
        let _stopwatch = Stopwatch::new(format!("Account #{} crawled", previous_info.id));

        match current_info {
            Some(current_info) => {
                self.crawl_existing_account(&mut *connection, previous_info, current_info)
                    .await?;
            }
            None => {
                log::warn!("Account #{} does not exist. Deleting…", previous_info.id);
                database::delete_account(&mut *connection, previous_info.id).await?;
            }
        };

        Ok(())
    }

    async fn crawl_existing_account(
        &self,
        connection: &mut PgConnection,
        previous_info: &BaseAccountInfo,
        mut current_info: AccountInfo,
    ) -> crate::Result {
        log::debug!("Crawling existing account #{}…", current_info.base.id);
        current_info.base.crawled_at = Utc::now();
        database::insert_account_or_replace(&mut *connection, &current_info.base).await?;

        if current_info.base.last_battle_time != previous_info.last_battle_time {
            let tanks: Vec<Tank> = self
                .api
                .get_merged_tanks(previous_info.id)
                .await?
                .into_iter()
                .filter(|tank| tank.last_battle_time > previous_info.last_battle_time)
                .collect();
            log::info!("Inserting tank snapshots for #{}…", previous_info.id);
            database::insert_tank_snapshots(&mut *connection, &tanks).await?;
        } else {
            log::debug!("No new battles detected.")
        }

        Ok(())
    }

    async fn retrieve_batch(&self, after: DateTime<Utc>) -> crate::Result<Vec<BaseAccountInfo>> {
        // language=SQL
        const QUERY: &str = r#"
        SELECT * FROM accounts
        WHERE crawled_at > $1
        ORDER BY crawled_at NULLS FIRST
        LIMIT $2
    "#;
        let accounts = sqlx::query_as(QUERY)
            .bind(after)
            .bind(100)
            .fetch_all(&self.database)
            .await
            .context("failed to retrieve a batch")?;
        Ok(accounts)
    }
}
