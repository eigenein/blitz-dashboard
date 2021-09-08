use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use redis::aio::ConnectionManager as Redis;
use sqlx::{PgConnection, PgPool};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::{log_metrics, SubCrawlerMetrics};
use crate::crawler::selector::Selector;
use crate::database;
use crate::database::models::{Account, AccountFactors};
use crate::database::retrieve_tank_ids;
use crate::math::{add_vector, dot, ensure_vector_length};
use crate::metrics::Stopwatch;
use crate::models::{merge_tanks, AccountInfo, Tank, TankStatistics};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::wargaming::WargamingApi;
use redis::AsyncCommands;

mod batch_stream;
mod metrics;
mod selector;

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    redis: Mutex<Redis>,

    n_tasks: usize,
    metrics: Arc<Mutex<SubCrawlerMetrics>>,

    /// Indicates that the accounts coming from the stream aren't real –
    /// they're generated on the fly instead of being read from the database.
    non_incremental: bool,

    /// Used to maintain the vehicles table in the database.
    /// The cache contains tank IDs which are for sure existing in the database at the moment.
    vehicle_cache: Arc<RwLock<HashSet<i32>>>,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(mut opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let api = new_wargaming_api(&opts.connections.application_id)?;
    let database = crate::database::open(
        &opts.connections.database_uri,
        opts.connections.initialize_schema,
    )
    .await?;
    let redis = crate::thirdparty::redis::open(&opts.connections.redis_uri).await?;

    opts.offsets.sort_unstable();
    let selectors = convert_offsets_to_selectors(&opts.offsets, opts.min_offset);
    for (i, selector) in selectors.iter().enumerate() {
        log::info!("Sub-crawler #{}: {}.", i, selector);
    }

    let (metrics, handles): (
        Vec<Arc<Mutex<SubCrawlerMetrics>>>,
        Vec<JoinHandle<crate::Result>>,
    ) = stream::iter(selectors)
        .then(|selector| {
            let api = api.clone();
            let database = database.clone();
            let redis = redis.clone();

            async move {
                let crawler = Crawler::new(api, database.clone(), redis, 1, false).await?;
                let metrics = crawler.metrics.clone();
                let join_handle = tokio::spawn(async move {
                    crawler
                        .run(get_batch_stream(database, selector))
                        .await
                        .with_context(|| format!("failed to run the sub-crawler: {}", selector))
                });
                Ok::<_, anyhow::Error>((metrics, join_handle))
            }
        })
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .unzip();
    tokio::spawn(log_metrics(api.request_counter, metrics));

    log::info!("Running…");
    futures::future::try_join_all(handles)
        .await?
        .into_iter()
        .collect()
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
    let database = crate::database::open(
        &opts.connections.database_uri,
        opts.connections.initialize_schema,
    )
    .await?;
    let redis = crate::thirdparty::redis::open(&opts.connections.redis_uri).await?;

    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(Account::empty)
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(api.clone(), database, redis, opts.n_tasks, true).await?;
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
fn convert_offsets_to_selectors(offsets: &[StdDuration], min_offset: StdDuration) -> Vec<Selector> {
    let mut selectors = Vec::new();
    let mut last_offset: Option<&StdDuration> = None;
    for offset in offsets {
        match last_offset {
            Some(last_offset) => selectors.push(Selector::Between(*last_offset, *offset)),
            None => selectors.push(Selector::Between(min_offset, *offset)),
        }
        last_offset = Some(offset);
    }
    match last_offset {
        Some(offset) => selectors.push(Selector::EarlierThan(*offset)),
        None => selectors.push(Selector::EarlierThan(min_offset)),
    }
    selectors
}

impl Crawler {
    pub async fn new(
        api: WargamingApi,
        database: PgPool,
        redis: Redis,
        n_tasks: usize,
        non_incremental: bool,
    ) -> crate::Result<Self> {
        let tank_ids: HashSet<i32> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            api,
            database,
            redis: Mutex::new(redis),
            n_tasks,
            non_incremental,
            metrics: Arc::new(Mutex::new(SubCrawlerMetrics::default())),
            vehicle_cache: Arc::new(RwLock::new(tank_ids)),
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(&self, stream: impl Stream<Item = crate::Result<Batch>>) -> crate::Result {
        stream
            .map(|batch| async move { self.crawl_batch(batch?, self.non_incremental).await })
            .buffer_unordered(self.n_tasks)
            .try_collect()
            .await
    }

    async fn crawl_batch(&self, batch: Batch, non_incremental: bool) -> crate::Result {
        let account_ids: Vec<i32> = batch.iter().map(|account| account.base.id).collect();
        let mut new_infos = self.api.get_account_info(&account_ids).await?;

        let mut tx = self.database.begin().await?;
        for account in batch.into_iter() {
            let account_id = account.base.id;
            let new_info = new_infos.remove(&account_id.to_string()).flatten();
            self.crawl_account(&mut tx, account, new_info, non_incremental)
                .await?;
            self.update_metrics_for_account(account_id).await;
        }
        log::debug!("Committing…");
        tx.commit().await?;

        Ok(())
    }

    async fn update_metrics_for_account(&self, account_id: i32) {
        let mut metrics = self.metrics.lock().await;
        metrics.last_account_id = account_id;
        metrics.n_accounts += 1;
    }

    async fn crawl_account(
        &self,
        connection: &mut PgConnection,
        account: Account,
        new_info: Option<AccountInfo>,
        non_incremental: bool,
    ) -> crate::Result {
        let _stopwatch = Stopwatch::new(format!("Account #{} crawled", account.base.id));

        if let Some(new_info) = new_info {
            self.crawl_existing_account(&mut *connection, account, new_info)
                .await?;
        } else if !non_incremental {
            Self::delete_account(&mut *connection, account.base.id).await?;
        }

        Ok(())
    }

    async fn delete_account(connection: &mut PgConnection, account_id: i32) -> crate::Result {
        log::warn!("Account #{} does not exist. Deleting…", account_id);
        database::delete_account(connection, account_id).await?;
        Ok(())
    }

    async fn crawl_existing_account(
        &self,
        connection: &mut PgConnection,
        account: Account,
        new_info: AccountInfo,
    ) -> crate::Result {
        if new_info.base.last_battle_time == account.base.last_battle_time {
            log::trace!("#{}: last battle time is not changed.", account.base.id);
            return Ok(());
        }

        let base = account.base;
        let mut cf = account.cf;
        log::debug!("Crawling account #{}…", base.id);
        let statistics = self
            .get_updated_tanks_statistics(base.id, base.last_battle_time)
            .await?;
        if !statistics.is_empty() {
            let achievements = self.api.get_tanks_achievements(base.id).await?;
            let tanks = merge_tanks(base.id, statistics, achievements);
            database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            self.insert_missing_vehicles(&mut *connection, &tanks)
                .await?;
            self.train(base.id, &mut cf, &tanks).await?;

            log::debug!("Inserted {} tanks for #{}.", tanks.len(), base.id);
            self.update_metrics_for_tanks(new_info.base.last_battle_time, tanks.len())
                .await?;
        } else {
            log::trace!("#{}: tanks are not updated.", base.id);
        }

        database::replace_account(
            &mut *connection,
            Account {
                base: new_info.base,
                cf,
            },
        )
        .await?;

        Ok(())
    }

    /// Gets account tanks which have their last battle time updated since the specified timestamp.
    async fn get_updated_tanks_statistics(
        &self,
        account_id: i32,
        since: DateTime<Utc>,
    ) -> crate::Result<Vec<TankStatistics>> {
        Ok(self
            .api
            .get_tanks_stats(account_id)
            .await?
            .into_iter()
            .filter(|tank| tank.base.last_battle_time > since)
            .collect())
    }

    async fn update_metrics_for_tanks(
        &self,
        last_battle_time: DateTime<Utc>,
        n_tanks: usize,
    ) -> crate::Result {
        let mut metrics = self.metrics.lock().await;
        metrics.push_lag((Utc::now() - last_battle_time).num_seconds().try_into()?);
        metrics.n_tanks += n_tanks;
        Ok(())
    }

    /// Inserts missing tank IDs into the database.
    async fn insert_missing_vehicles(
        &self,
        connection: &mut PgConnection,
        tanks: &[Tank],
    ) -> crate::Result {
        for tank in tanks {
            let tank_id = tank.statistics.base.tank_id;
            if !self.vehicle_cache.read().await.contains(&tank_id) {
                self.vehicle_cache.write().await.insert(tank_id);
                database::insert_vehicle_or_ignore(&mut *connection, tank_id).await?;
            }
        }
        Ok(())
    }

    /// Trains the account and vehicle factors on the new data.
    /// Implements a stochastic gradient descent for matrix factorization.
    ///
    /// https://blog.insightdatascience.com/explicit-matrix-factorization-als-sgd-and-all-that-jazz-b00e4d9b21ea
    async fn train(
        &self,
        account_id: i32,
        account: &mut AccountFactors,
        tanks: &[Tank],
    ) -> crate::Result {
        const GLOBAL_BIAS: f64 = 0.5;
        const ACCOUNT_LEARNING_RATE: f64 = 0.01;
        const VEHICLE_LEARNING_RATE: f64 = 0.001;
        const N_FACTORS: usize = 8;

        ensure_vector_length(&mut account.factors, N_FACTORS);

        for tank in tanks {
            let tank_id = tank.statistics.base.tank_id;
            let vehicle_key = format!("t:{}:factors", tank_id);

            // Read the vehicle profile and initialize it, if needed.
            let mut redis = self.redis.lock().await;
            let mut vehicle_factors: Vec<f64> = redis
                .get::<'_, _, Option<Vec<u8>>>(&vehicle_key)
                .await?
                .map(|factors| rmp_serde::from_read_ref(&factors))
                .unwrap_or_else(|| Ok(Vec::new()))?;
            ensure_vector_length(&mut vehicle_factors, N_FACTORS + 1);

            // Let's see how much battles and losses the account has made.
            let (n_battles, n_wins) =
                database::retrieve_tank_battle_count(&self.database, account_id, tank_id).await?;
            let n_wins = tank.statistics.all.wins - n_wins;
            let n_battles = tank.statistics.all.battles - n_battles;
            assert!(
                n_battles >= 0,
                "account_id: {}, tank_id: {}, all_battles: {}, n_battles: {}",
                account_id,
                tank_id,
                tank.statistics.all.battles,
                n_battles,
            );

            let mut metrics = self.metrics.lock().await;
            metrics.n_battles += n_battles;

            // For each battle doing the SGD step.
            for i in 0..n_battles {
                let outcome = if i < n_wins { 1.0 } else { 0.0 };
                let prediction = GLOBAL_BIAS
                    + account.bias
                    + vehicle_factors[0]
                    + dot(&account.factors, &vehicle_factors[1..]);
                let error = prediction - outcome;
                metrics.cf_error += error;

                account.bias += ACCOUNT_LEARNING_RATE * (error - account.bias);
                vehicle_factors[0] += VEHICLE_LEARNING_RATE * (error - vehicle_factors[0]);

                add_vector(
                    &mut account.factors,
                    &vehicle_factors[1..],
                    ACCOUNT_LEARNING_RATE * error,
                );
                add_vector(
                    &mut vehicle_factors[1..],
                    &account.factors,
                    VEHICLE_LEARNING_RATE * error,
                )
            }

            log::trace!("Vehicle #{} factors: {:?}.", tank_id, vehicle_factors);
            redis
                .set(&vehicle_key, rmp_serde::to_vec(&vehicle_factors)?)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN_OFFSET: StdDuration = StdDuration::from_secs(1);

    #[test]
    fn convert_no_offsets_ok() {
        assert_eq!(
            convert_offsets_to_selectors(&[], MIN_OFFSET),
            vec![Selector::EarlierThan(MIN_OFFSET)],
        );
    }

    #[test]
    fn convert_one_offset_ok() {
        let offset = StdDuration::from_secs(2);
        assert_eq!(
            convert_offsets_to_selectors(&[offset], MIN_OFFSET),
            vec![
                Selector::Between(MIN_OFFSET, offset),
                Selector::EarlierThan(offset),
            ]
        );
    }

    #[test]
    fn convert_two_offsets_ok() {
        let offset_1 = StdDuration::from_secs(2);
        let offset_2 = StdDuration::from_secs(3);
        assert_eq!(
            convert_offsets_to_selectors(&[offset_1, offset_2], MIN_OFFSET),
            vec![
                Selector::Between(MIN_OFFSET, offset_1),
                Selector::Between(offset_1, offset_2),
                Selector::EarlierThan(offset_2),
            ]
        );
    }
}
