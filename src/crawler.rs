use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{TimeZone, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use smallvec::SmallVec;
use sqlx::{PgConnection, PgPool};
use tokio::sync::RwLock;

use crate::crawler::batch_stream::{loop_batches_from, Batch, Select};
use crate::crawler::metrics::{CrawlerMetrics, TotalCrawlerMetrics};
use crate::database;
use crate::database::retrieve_tank_ids;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BaseAccountInfo, Tank};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::wargaming::WargamingApi;

mod batch_stream;
mod metrics;

pub async fn run_crawler(opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let metrics = TotalCrawlerMetrics::new();
    let api = new_wargaming_api(
        &opts.crawler.connections.application_id,
        metrics.n_api_requests.clone(),
    )?;
    let database = crate::database::open(&opts.crawler.connections.database).await?;
    let hot_crawler = Crawler::new(api.clone(), database.clone(), metrics.hot.clone()).await?;
    let cold_crawler = Crawler::new(api, database.clone(), metrics.cold.clone()).await?;

    log::info!("Starting…");
    futures::future::try_join3(
        hot_crawler.run(
            loop_batches_from(database.clone(), Select::Hot, opts.hot_offset),
            opts.n_hot_tasks,
            false,
        ),
        cold_crawler.run(
            loop_batches_from(database.clone(), Select::Cold, opts.cold_offset),
            opts.crawler.n_cold_tasks,
            false,
        ),
        log_metrics(metrics),
    )
    .await?;

    Ok(())
}

pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let metrics = TotalCrawlerMetrics::new();
    let api = new_wargaming_api(
        &opts.crawler.connections.application_id,
        metrics.n_api_requests.clone(),
    )?;
    let database = crate::database::open(&opts.crawler.connections.database).await?;
    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(|account_id| BaseAccountInfo {
            id: account_id,
            last_battle_time: Utc.timestamp(0, 0),
        })
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(api, database, metrics.cold.clone()).await?;
    futures::future::try_join(
        crawler.run(stream, opts.crawler.n_cold_tasks, true),
        log_metrics(metrics),
    )
    .await?;
    Ok(())
}

async fn log_metrics(mut metrics: TotalCrawlerMetrics) -> crate::Result {
    loop {
        metrics.log();
        tokio::time::sleep(StdDuration::from_secs(20)).await;
    }
}

fn new_wargaming_api(
    application_id: &str,
    request_counter: Arc<AtomicU32>,
) -> crate::Result<WargamingApi> {
    WargamingApi::new(
        application_id,
        StdDuration::from_millis(3000),
        request_counter,
    )
}

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    vehicle_cache: Arc<RwLock<HashSet<i32>>>,
    metrics: CrawlerMetrics,
}

impl Crawler {
    pub async fn new(
        api: WargamingApi,
        database: PgPool,
        metrics: CrawlerMetrics,
    ) -> crate::Result<Self> {
        let tank_ids: HashSet<i32> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            api,
            database,
            metrics,
            vehicle_cache: Arc::new(RwLock::new(tank_ids)),
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(
        &self,
        stream: impl Stream<Item = crate::Result<Batch>>,
        n_tasks: usize,
        fake_infos: bool,
    ) -> crate::Result {
        stream
            .map(|batch| async move { self.clone().crawl_batch(batch?, fake_infos).await })
            .buffer_unordered(n_tasks)
            .try_collect()
            .await
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
            self.metrics
                .last_account_id
                .swap(old_info.id, Ordering::Relaxed);
            self.metrics.n_accounts.fetch_add(1, Ordering::Relaxed);
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
            database::insert_tank_snapshots(&mut *connection, &tanks, self.metrics.n_tanks.clone())
                .await?;
            self.insert_vehicles(&mut *connection, &tanks).await?;
            log::debug!("Inserted {} tanks for #{}.", tanks.len(), old_info.id);
            self.metrics.max_lag_secs.fetch_max(
                (Utc::now() - new_info.base.last_battle_time)
                    .num_seconds()
                    .try_into()?,
                Ordering::Relaxed,
            );
        } else {
            log::debug!("Account #{} haven't played.", old_info.id)
        }

        Ok(())
    }

    /// Inserts missing tank IDs into the database.
    async fn insert_vehicles(
        &self,
        connection: &mut PgConnection,
        tanks: &[Tank],
    ) -> crate::Result {
        for tank in tanks {
            if !self.vehicle_cache.read().await.contains(&tank.tank_id) {
                self.vehicle_cache.write().await.insert(tank.tank_id);
                database::insert_vehicle_or_ignore(&mut *connection, tank.tank_id).await?;
            }
        }
        Ok(())
    }
}
