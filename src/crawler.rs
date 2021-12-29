use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{Duration, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use redis::aio::MultiplexedConnection;
use sqlx::{PgConnection, PgPool};
use tokio::sync::RwLock;

use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::CrawlerMetrics;
use crate::database::{
    insert_tank_snapshots, insert_vehicle_or_ignore, open as open_database, replace_account,
    retrieve_tank_battle_count, retrieve_tank_ids,
};
use crate::metrics::Stopwatch;
use crate::models::{merge_tanks, AccountInfo, BaseAccountInfo, Tank, TankStatistics};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::trainer::dataset::push_stream_entries;
use crate::trainer::stream_entry::StreamEntry;
use crate::wargaming::tank_id::TankId;
use crate::wargaming::WargamingApi;
use crate::DateTime;

pub mod batch_stream;
mod metrics;

const API_TIMEOUT: StdDuration = StdDuration::from_secs(30);

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    redis: MultiplexedConnection,

    n_tasks: usize,
    metrics: CrawlerMetrics,
    auto_min_offset: Option<Arc<RwLock<StdDuration>>>,

    /// `Some(...)` indicates that only tanks with updated last battle time must be crawled.
    /// This also sends out updated tanks to the trainer.
    incremental: Option<IncrementalOpts>,

    /// Used to maintain the vehicle table in the database.
    /// The cache contains tank IDs which are for sure existing at the moment in the database.
    vehicle_cache: HashSet<TankId>,
}

#[derive(Copy, Clone)]
pub struct IncrementalOpts {
    training_stream_size: usize,
    training_stream_duration: Duration,
    test_percentage: usize,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let api = WargamingApi::new(&opts.connections.application_id, API_TIMEOUT)?;
    let internal = opts.connections.internal;
    let database = open_database(&internal.database_uri, internal.initialize_schema).await?;
    let redis = redis::Client::open(internal.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;

    let min_offset = Arc::new(RwLock::new(opts.min_offset));
    let crawler = Crawler::new(
        api,
        database.clone(),
        redis.clone(),
        opts.n_tasks,
        Some(IncrementalOpts {
            training_stream_size: opts.training_stream_size,
            training_stream_duration: opts.training_stream_duration,
            test_percentage: opts.test_percentage,
        }),
        opts.log_interval,
        opts.auto_min_offset.then(|| min_offset.clone()),
    )
    .await?;

    tracing::info!("running…");
    let accounts = Box::pin(get_batch_stream(database, redis, min_offset).await);
    crawler.run(accounts).await
}

/// Performs a very slow one-time account scan.
/// Spawns a crawler which unconditionally inserts and/or updates
/// accounts in the specified range.
///
/// This is a technical script which is intended to be run one time for an entire region
/// to populate the database.
pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let api = WargamingApi::new(&opts.connections.application_id, API_TIMEOUT)?;
    let internal = opts.connections.internal;
    let database = open_database(&internal.database_uri, internal.initialize_schema).await?;
    let redis = redis::Client::open(internal.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;

    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(BaseAccountInfo::empty)
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(
        api,
        database,
        redis,
        opts.n_tasks,
        None,
        opts.log_interval,
        None,
    )
    .await?;
    crawler.run(stream).await
}

impl Crawler {
    pub async fn new(
        api: WargamingApi,
        database: PgPool,
        redis: MultiplexedConnection,
        n_tasks: usize,
        incremental: Option<IncrementalOpts>,
        log_interval: StdDuration,
        auto_min_offset: Option<Arc<RwLock<StdDuration>>>,
    ) -> crate::Result<Self> {
        let tank_ids: HashSet<TankId> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            metrics: CrawlerMetrics::new(api.request_counter.clone(), log_interval),
            api,
            database,
            redis,
            n_tasks,
            incremental,
            auto_min_offset,
            vehicle_cache: tank_ids,
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(
        mut self,
        accounts: impl Stream<Item = crate::Result<Batch>> + Unpin,
    ) -> crate::Result {
        let api = self.api.clone();

        let mut accounts = accounts
            // Get account info for all accounts in the batch.
            .map_ok(|batch| async {
                let account_ids: Vec<i32> = batch.iter().map(|account| account.id).collect();
                let new_infos = api.get_account_info(&account_ids).await?;
                Ok((batch, new_infos))
            })
            // Parallel the API calls from above.
            .try_buffer_unordered(self.n_tasks)
            // Match the account infos against the accounts from the batch.
            .map_ok(|(batch, new_infos)| Self::zip_account_infos(batch, new_infos))
            // Make it a stream of accounts instead of the stream of batches.
            .try_flatten();

        while let Some((account, new_info)) = accounts.try_next().await? {
            self.metrics.add_account(account.id);
            self.crawl_account(account, new_info).await?;
            self.metrics.check(&self.auto_min_offset).await;
        }
        Ok(())
    }

    /// Match the batch's accounts to the account infos fetched from the API.
    ///
    /// Returns a stream of matched pairs.
    fn zip_account_infos(
        batch: Batch,
        mut new_infos: HashMap<String, Option<AccountInfo>>,
    ) -> impl Stream<Item = crate::Result<(BaseAccountInfo, AccountInfo)>> {
        let accounts = batch.into_iter().filter_map(move |account| {
            // Try and find the account in the new account infos.
            let new_info = new_infos.remove(&account.id.to_string()).flatten();
            // If found, return the matched pair.
            new_info.map(|new_info| Ok((account, new_info)))
        });
        stream::iter(accounts)
    }

    #[tracing::instrument(skip_all)]
    async fn crawl_account(
        &mut self,
        account: BaseAccountInfo,
        new_info: AccountInfo,
    ) -> crate::Result<bool> {
        let _stopwatch = Stopwatch::new(format!("account #{} crawled", account.id));

        if new_info.base.last_battle_time == account.last_battle_time {
            tracing::trace!(account_id = account.id, "last battle time unchanged");
            return Ok(false);
        }

        tracing::debug!(
            account_id = account.id,
            since = account.last_battle_time.to_rfc3339().as_str(),
            "crawling…",
        );
        let statistics = self
            .get_updated_tanks_statistics(account.id, account.last_battle_time)
            .await?;
        let mut transaction = self.database.begin().await?;
        if !statistics.is_empty() {
            let achievements = self.api.get_tanks_achievements(account.id).await?;
            let tanks = merge_tanks(account.id, statistics, achievements);
            insert_tank_snapshots(&mut transaction, &tanks).await?;
            self.insert_missing_vehicles(&mut transaction, &tanks)
                .await?;

            tracing::debug!(account_id = account.id, n_tanks = tanks.len(), "inserted");
            self.metrics
                .add_tanks(new_info.base.last_battle_time, tanks.len())?;

            if let Some(opts) = self.incremental {
                // Zero timestamp means that the account has never played or been crawled before.
                // FIXME: make the `last_battle_time` nullable instead.
                if account.last_battle_time.timestamp() != 0 {
                    self.push_incremental_updates(&opts, account.id, &tanks)
                        .await?;
                }
            }
        } else {
            tracing::debug!(account_id = account.id, "no updated tanks");
        }

        replace_account(&mut transaction, &new_info.base).await?;
        transaction
            .commit()
            .await
            .with_context(|| format!("failed to commit account #{}", account.id))?;

        tracing::debug!(account_id = account.id, "done");
        Ok(true)
    }

    /// Gets account tanks which have their last battle time updated since the specified timestamp.
    async fn get_updated_tanks_statistics(
        &self,
        account_id: i32,
        since: DateTime,
    ) -> crate::Result<Vec<TankStatistics>> {
        Ok(self
            .api
            .get_tanks_stats(account_id)
            .await?
            .into_iter()
            .filter(|tank| tank.base.last_battle_time > since)
            .collect())
    }

    /// Inserts missing tank IDs into the database.
    async fn insert_missing_vehicles(
        &mut self,
        connection: &mut PgConnection,
        tanks: &[Tank],
    ) -> crate::Result {
        for tank in tanks {
            let tank_id = tank.statistics.base.tank_id;
            if !self.vehicle_cache.contains(&tank_id) {
                self.vehicle_cache.insert(tank_id);
                insert_vehicle_or_ignore(&mut *connection, tank_id).await?;
            }
        }
        Ok(())
    }

    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(account_id = account_id, n_tanks = tanks.len()),
    )]
    async fn prepare_stream_entries(
        &mut self,
        account_id: i32,
        tanks: &[Tank],
        opts: &IncrementalOpts,
    ) -> crate::Result<Vec<StreamEntry>> {
        let now = Utc::now();
        let mut entries = Vec::new();

        for tank in tanks {
            let last_battle_time = tank.statistics.base.last_battle_time;
            if now - last_battle_time > opts.training_stream_duration {
                tracing::debug!(tank_id = tank.tank_id(), "the last battle is too old");
                continue;
            }
            let tank_id = tank.statistics.base.tank_id;
            let (n_battles, n_wins) =
                retrieve_tank_battle_count(&self.database, account_id, tank_id).await?;
            let n_battles = tank.statistics.all.battles - n_battles;
            let n_wins = tank.statistics.all.wins - n_wins;
            if n_battles > 0 && n_wins >= 0 {
                self.metrics.add_battles(n_battles);
                entries.push(StreamEntry {
                    account_id,
                    tank_id,
                    timestamp: last_battle_time.timestamp(),
                    n_battles,
                    n_wins,
                    is_test: fastrand::usize(0..100) < opts.test_percentage,
                });
            }
        }

        Ok(entries)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn push_incremental_updates(
        &mut self,
        opts: &IncrementalOpts,
        account_id: i32,
        tanks: &[Tank],
    ) -> crate::Result {
        let entries = self.prepare_stream_entries(account_id, tanks, opts).await?;
        if !entries.is_empty() {
            push_stream_entries(
                &mut self.redis.clone(),
                &entries,
                opts.training_stream_size,
                opts.training_stream_duration,
            )
            .await?;
        }
        Ok(())
    }
}
