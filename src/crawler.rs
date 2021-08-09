use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{TimeZone, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use smallvec::SmallVec;
use sqlx::{PgConnection, PgPool};
use tokio::sync::RwLock;

use crate::crawler::batch_stream::{loop_batches_from, Batch};
use crate::database;
use crate::database::{retrieve_max_account_id, retrieve_tank_ids};
use crate::metrics::{RpsCounter, Stopwatch};
use crate::models::{AccountInfo, BaseAccountInfo, Tank};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::wargaming::WargamingApi;

mod batch_stream;

pub async fn run_crawler(opts: CrawlerOpts) -> crate::Result {
    let api = new_wargaming_api(&opts.application_id)?;
    let database = crate::database::open(&opts.database).await?;
    let starting_account_id = fastrand::i32(0..retrieve_max_account_id(&database).await?);
    log::info!("Starting the crawler from #{}…", starting_account_id);
    let stream = loop_batches_from(database.clone(), starting_account_id);
    Crawler::new(api, database)
        .await?
        .run(stream, opts.n_tasks, false)
        .await
}

pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let api = new_wargaming_api(&opts.application_id)?;
    let database = crate::database::open(&opts.database).await?;
    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(|account_id| BaseAccountInfo {
            id: account_id,
            last_battle_time: Utc.timestamp(0, 0),
        })
        .chunks(100)
        .map(|batch| Ok(batch));
    Crawler::new(api, database)
        .await?
        .run(stream, opts.n_tasks, true)
        .await
}

fn new_wargaming_api(application_id: &str) -> crate::Result<WargamingApi> {
    WargamingApi::new(application_id, StdDuration::from_millis(1500))
}

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    vehicle_cache: Arc<RwLock<HashSet<i32>>>,
    account_rps_counter: Arc<RwLock<RpsCounter>>,
}

impl Crawler {
    pub async fn new(api: WargamingApi, database: PgPool) -> crate::Result<Self> {
        let tank_ids: HashSet<i32> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            api,
            database,
            vehicle_cache: Arc::new(RwLock::new(tank_ids)),
            account_rps_counter: Arc::new(RwLock::new(RpsCounter::new("Accounts", 2000))),
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
        sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

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
            self.account_rps_counter.write().await.increment();
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
            log::info!("Inserted {} tanks for #{}.", tanks.len(), old_info.id);
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
