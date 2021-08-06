use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{TimeZone, Utc};
use futures::future::select_all;
use itertools::Itertools;
use smallvec::SmallVec;
use sqlx::{PgConnection, PgPool};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::database;
use crate::database::retrieve_max_account_id;
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
        let old_infos: Vec<BaseAccountInfo> = chunk
            .map(|account_id| BaseAccountInfo {
                id: account_id,
                last_battle_time: epoch,
            })
            .collect();
        crawler.clone().crawl_batch(old_infos, true).await?;
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

        let mut metrics_start = Instant::now();
        let mut account_pointer = fastrand::i32(0..retrieve_max_account_id(&self.database).await?);
        let mut running_futures: Vec<JoinHandle<crate::Result>> = Vec::new();

        log::info!("Starting the crawler from #{}…", account_pointer);

        loop {
            if running_futures.len() < n_tasks {
                let batch = self.retrieve_batch(account_pointer).await?;
                match batch.last() {
                    Some(info) => {
                        account_pointer = info.id;
                        running_futures.push(tokio::spawn(self.clone().crawl_batch(batch, false)));
                    }
                    None => {
                        log::info!("Starting over.");
                        account_pointer = 0
                    }
                }
            } else {
                let (resolved_future, _, remaining_futures) = select_all(running_futures).await;
                resolved_future??;
                running_futures = remaining_futures;
            }
            if metrics_start.elapsed().as_secs() > 10 {
                let elapsed = metrics_start.elapsed().as_secs_f64();
                let rps = self.api.get_request_counter() as f64 / elapsed;
                log::info!("Rate: {:.1} RPS.", rps);
                metrics_start = Instant::now();
                self.api.reset_request_counter();
            }
        }
    }

    async fn crawl_batch(self, old_infos: Vec<BaseAccountInfo>, fake_infos: bool) -> crate::Result {
        let account_ids: SmallVec<[i32; 128]> =
            old_infos.iter().map(|account| account.id).collect();
        let mut new_infos = self.api.get_account_info(&account_ids).await?;

        let mut tx = self.database.begin().await?;
        for old_info in old_infos.iter() {
            let new_info = new_infos.remove(&old_info.id.to_string()).flatten();
            self.maybe_crawl_account(&mut tx, old_info, new_info, fake_infos)
                .await?;
        }
        log::debug!("Committing…");
        tx.commit().await?;

        Ok(())
    }

    async fn maybe_crawl_account(
        &self,
        connection: &mut PgConnection,
        old_info: &BaseAccountInfo,
        new_info: Option<AccountInfo>,
        fake_info: bool,
    ) -> crate::Result {
        let _stopwatch = Stopwatch::new(format!("Account #{} crawled", old_info.id));

        match new_info {
            Some(new_info) => {
                self.crawl_existing_account(&mut *connection, old_info, new_info)
                    .await?;
            }
            None => {
                if !fake_info {
                    log::warn!("Account #{} does not exist. Deleting…", old_info.id);
                    database::delete_account(&mut *connection, old_info.id).await?;
                }
            }
        };

        Ok(())
    }

    async fn crawl_existing_account(
        &self,
        connection: &mut PgConnection,
        old_info: &BaseAccountInfo,
        new_info: AccountInfo,
    ) -> crate::Result {
        if new_info.base.last_battle_time != old_info.last_battle_time {
            log::debug!("Crawling account #{}…", old_info.id);
            database::insert_account_or_replace(&mut *connection, &new_info.base).await?;
            let tanks: Vec<Tank> = self
                .api
                .get_merged_tanks(old_info.id)
                .await?
                .into_iter()
                .filter(|tank| tank.last_battle_time > old_info.last_battle_time)
                .collect();
            database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            log::info!("Inserted {} tanks for #{}.", tanks.len(), old_info.id);
        } else {
            log::debug!("Account #{} haven't played.", old_info.id)
        }

        Ok(())
    }

    async fn retrieve_batch(&self, pointer: i32) -> crate::Result<Vec<BaseAccountInfo>> {
        // language=SQL
        const QUERY: &str = r#"
            SELECT * FROM accounts
            WHERE account_id > $1
            ORDER BY account_id 
            LIMIT 100
        "#;
        let accounts = sqlx::query_as(QUERY)
            .bind(pointer)
            .fetch_all(&self.database)
            .await
            .context("failed to retrieve a batch")?;
        Ok(accounts)
    }
}
