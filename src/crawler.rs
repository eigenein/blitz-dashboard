use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{TimeZone, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use smallvec::SmallVec;
use sqlx::{PgConnection, PgPool};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::{log_metrics, SubCrawlerMetrics};
use crate::crawler::selector::Selector;
use crate::database;
use crate::database::retrieve_tank_ids;
use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BaseAccountInfo, Tank};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::wargaming::WargamingApi;

mod batch_stream;
mod metrics;
mod selector;

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    n_tasks: usize,
    fake_infos: bool,
    vehicle_cache: Arc<RwLock<HashSet<i32>>>,
    metrics: Arc<Mutex<SubCrawlerMetrics>>,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(mut opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let api = new_wargaming_api(&opts.connections.application_id)?;
    let database = crate::database::open(&opts.connections.database_uri).await?;

    opts.offsets.sort_unstable();
    let selectors = convert_offsets_to_selectors(&opts.offsets);
    for (i, selector) in selectors.iter().enumerate() {
        log::info!("Sub-crawler #{}: {}.", i, selector);
    }

    log::info!("--------------------------------------------------------------------------------");

    let (metrics, handles): (
        Vec<Arc<Mutex<SubCrawlerMetrics>>>,
        Vec<JoinHandle<crate::Result>>,
    ) = stream::iter(selectors)
        .then(|selector| {
            let api = api.clone();
            let database = database.clone();
            async move {
                let crawler = Crawler::new(api, database.clone(), 1, false).await?;
                let metrics = crawler.metrics.clone();
                let join_handle = tokio::spawn(async move {
                    let stream = get_batch_stream(database, selector);
                    crawler.run(stream).await
                });
                Ok::<_, anyhow::Error>((metrics, join_handle))
            }
        })
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .unzip();
    tokio::spawn(log_metrics(api.request_counter, metrics));

    futures::future::try_join_all(handles).await?;
    Ok(())
}

/// Performs a very slow one-time account scan.
/// Spawns a single sub-crawler which unconditionally inserts and/or updates
/// accounts in the specified range.
///
/// This is a technical script which is intended to be run one time for an entire region
/// to populate the database.
pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let api = new_wargaming_api(&opts.connections.application_id)?;
    let database = crate::database::open(&opts.connections.database_uri).await?;
    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(|account_id| BaseAccountInfo {
            id: account_id,
            last_battle_time: Utc.timestamp(0, 0),
        })
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(api.clone(), database, opts.n_tasks, true).await?;
    tokio::spawn(log_metrics(
        api.request_counter.clone(),
        vec![crawler.metrics.clone()],
    ));
    tokio::spawn(async move { crawler.run(stream).await }).await??;
    Ok(())
}

fn new_wargaming_api(application_id: &str) -> crate::Result<WargamingApi> {
    WargamingApi::new(application_id, StdDuration::from_millis(3000))
}

/// Converts user-defined offsets to sub-crawler selectors.
fn convert_offsets_to_selectors(offsets: &[StdDuration]) -> Vec<Selector> {
    let mut selectors = Vec::new();
    let mut last_offset: Option<&StdDuration> = None;
    for offset in offsets {
        match last_offset {
            Some(last_offset) => selectors.push(Selector::Between(*last_offset, *offset)),
            None => selectors.push(Selector::LaterThan(*offset)),
        }
        last_offset = Some(offset);
    }
    match last_offset {
        Some(offset) => selectors.push(Selector::EarlierThan(*offset)),
        None => selectors.push(Selector::All),
    }
    selectors
}

impl Crawler {
    pub async fn new(
        api: WargamingApi,
        database: PgPool,
        n_tasks: usize,
        fake_infos: bool,
    ) -> crate::Result<Self> {
        let tank_ids: HashSet<i32> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            api,
            database,
            n_tasks,
            fake_infos,
            metrics: Arc::new(Mutex::new(SubCrawlerMetrics::default())),
            vehicle_cache: Arc::new(RwLock::new(tank_ids)),
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(&self, stream: impl Stream<Item = crate::Result<Batch>>) -> crate::Result {
        stream
            .map(|batch| async move { self.crawl_batch(batch?, self.fake_infos).await })
            .buffer_unordered(self.n_tasks)
            .try_collect()
            .await
    }

    async fn crawl_batch(
        &self,
        old_infos: Vec<BaseAccountInfo>,
        fake_infos: bool,
    ) -> crate::Result {
        let account_ids: SmallVec<[i32; 128]> =
            old_infos.iter().map(|account| account.id).collect();
        let mut new_infos = self.api.get_account_info(&account_ids).await?;

        let mut tx = self.database.begin().await?;
        for old_info in old_infos.iter() {
            let new_info = new_infos.remove(&old_info.id.to_string()).flatten();
            self.maybe_crawl_account(&mut tx, old_info, new_info, fake_infos)
                .await?;
            {
                let mut metrics = self.metrics.lock().await;
                metrics.last_account_id = old_info.id;
                metrics.n_accounts += 1;
            }
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
            self.insert_vehicles(&mut *connection, &tanks).await?;

            log::debug!("Inserted {} tanks for #{}.", tanks.len(), old_info.id);
            {
                let mut metrics = self.metrics.lock().await;
                let lag_secs = (Utc::now() - new_info.base.last_battle_time)
                    .num_seconds()
                    .try_into()?;
                metrics.max_lag_secs = metrics.max_lag_secs.max(lag_secs);
                metrics.n_tanks += tanks.len();
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_no_offsets_ok() {
        assert_eq!(convert_offsets_to_selectors(&[]), vec![Selector::All]);
    }

    #[test]
    fn convert_one_offset_ok() {
        let offset = StdDuration::from_secs(1);
        assert_eq!(
            convert_offsets_to_selectors(&[offset]),
            vec![Selector::LaterThan(offset), Selector::EarlierThan(offset)]
        );
    }

    #[test]
    fn convert_two_offsets_ok() {
        let offset_1 = StdDuration::from_secs(1);
        let offset_2 = StdDuration::from_secs(2);
        assert_eq!(
            convert_offsets_to_selectors(&[offset_1, offset_2]),
            vec![
                Selector::LaterThan(offset_1),
                Selector::Between(offset_1, offset_2),
                Selector::EarlierThan(offset_2),
            ]
        );
    }
}
