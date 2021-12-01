use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::Context;
use arc_swap::ArcSwap;
use chrono::{TimeZone, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands};
use sqlx::{PgConnection, PgPool};
use tokio::sync::{Mutex, RwLock};

use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::{log_metrics, CrawlerMetrics};
use crate::database::{
    insert_tank_snapshots, insert_vehicle_or_ignore, open as open_database, replace_account,
    retrieve_tank_battle_count, retrieve_tank_ids,
};
use crate::metrics::Stopwatch;
use crate::models::{merge_tanks, AccountInfo, BaseAccountInfo, Tank, TankStatistics};
use crate::opts::{CrawlAccountsOpts, CrawlerOpts};
use crate::trainer::push_sample_points;
use crate::trainer::sample_point::SamplePoint;
use crate::wargaming::WargamingApi;
use crate::DateTime;

mod batch_stream;
mod metrics;

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    redis: MultiplexedConnection,

    n_tasks: usize,
    metrics: Arc<Mutex<CrawlerMetrics>>,

    /// `Some(...)` indicates that only tanks with updated last battle time must be crawled.
    /// This also sends out updated tanks to the trainer.
    incremental: Option<IncrementalOpts>,

    /// Used to maintain the vehicle table in the database.
    /// The cache contains tank IDs which are for sure existing at the moment in the database.
    vehicle_cache: Arc<RwLock<HashSet<i32>>>,
}

pub struct IncrementalOpts {
    training_stream_size: usize,
    test_percentage: usize,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let api = WargamingApi::new(&opts.connections.application_id)?;
    let request_counter = api.request_counter.clone();
    let internal = opts.connections.internal;
    let database = open_database(&internal.database_uri, internal.initialize_schema).await?;
    let redis = redis::Client::open(internal.redis_uri.as_str())?;

    let crawler = Crawler::new(
        api,
        database.clone(),
        redis.get_multiplexed_async_connection().await?,
        opts.n_tasks,
        Some(IncrementalOpts {
            training_stream_size: opts.training_stream_size,
            test_percentage: opts.test_percentage,
        }),
    )
    .await?;

    tracing::info!("running…");
    // TODO: just use `AtomicU64` instead to store the offset in seconds.
    let min_offset = Arc::new(ArcSwap::new(Arc::new(opts.min_offset)));
    tokio::spawn(log_metrics(
        request_counter,
        crawler.metrics.clone(),
        opts.log_interval,
        if opts.auto_min_offset {
            Some(Arc::clone(&min_offset))
        } else {
            None
        },
    ));
    crawler
        .run(get_batch_stream(database.clone(), min_offset))
        .await?;

    Ok(())
}

/// Performs a very slow one-time account scan.
/// Spawns a crawler which unconditionally inserts and/or updates
/// accounts in the specified range.
///
/// This is a technical script which is intended to be run one time for an entire region
/// to populate the database.
pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let api = WargamingApi::new(&opts.connections.application_id)?;
    let internal = opts.connections.internal;
    let database = open_database(&internal.database_uri, internal.initialize_schema).await?;
    let redis = redis::Client::open(internal.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;

    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(BaseAccountInfo::empty)
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(api.clone(), database, redis, opts.n_tasks, None).await?;
    tokio::spawn(log_metrics(
        api.request_counter.clone(),
        crawler.metrics.clone(),
        StdDuration::from_secs(60),
        None,
    ));
    crawler.run(stream).await?;
    Ok(())
}

impl Crawler {
    pub async fn new(
        api: WargamingApi,
        database: PgPool,
        redis: MultiplexedConnection,
        n_tasks: usize,
        incremental: Option<IncrementalOpts>,
    ) -> crate::Result<Self> {
        let tank_ids: HashSet<i32> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            api,
            database,
            redis,
            n_tasks,
            incremental,
            metrics: Arc::new(Mutex::new(CrawlerMetrics::default())),
            vehicle_cache: Arc::new(RwLock::new(tank_ids)),
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(&self, stream: impl Stream<Item = crate::Result<Batch>>) -> crate::Result {
        stream
            .map(|batch| async move { self.crawl_batch(batch?).await })
            .buffer_unordered(self.n_tasks)
            .try_collect()
            .await
    }

    async fn crawl_batch(&self, batch: Batch) -> crate::Result {
        let account_ids: Vec<i32> = batch.iter().map(|account| account.id).collect();
        let mut new_infos = self.api.get_account_info(&account_ids).await?;

        for account in batch.into_iter() {
            let account_id = account.id;
            if let Some(new_info) = new_infos.remove(&account.id.to_string()).flatten() {
                self.crawl_account(account, new_info).await?;
            }
            self.update_account_metrics(account_id).await;
        }

        Ok(())
    }

    async fn update_account_metrics(&self, account_id: i32) {
        let mut metrics = self.metrics.lock().await;
        metrics.last_account_id = account_id;
        metrics.n_accounts += 1;
    }

    #[tracing::instrument(skip_all)]
    async fn crawl_account(
        &self,
        account: BaseAccountInfo,
        new_info: AccountInfo,
    ) -> crate::Result {
        let _stopwatch = Stopwatch::new(format!("account #{} crawled", account.id));

        if new_info.base.last_battle_time == account.last_battle_time {
            // TODO: only if non-incremental.
            set_account_last_battle_time(&self.redis, &new_info.base).await?;

            tracing::trace!(account_id = account.id, "last battle time unchanged");
            return Ok(());
        }

        tracing::debug!(account_id = account.id, "crawling…");
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
            self.update_tank_metrics(new_info.base.last_battle_time, tanks.len())
                .await?;

            if let Some(opts) = &self.incremental {
                // Zero timestamp means that the account has never played or been crawled before.
                // FIXME: make the `last_battle_time` nullable instead.
                if account.last_battle_time.timestamp() != 0 {
                    self.push_sample_points(account.id, &tanks, opts).await?;
                }
            }
        } else {
            tracing::trace!(account_id = account.id, "no updated tanks");
        }

        replace_account(&mut transaction, &new_info.base).await?;
        transaction
            .commit()
            .await
            .with_context(|| format!("failed to commit account #{}", account.id))?;
        set_account_last_battle_time(&self.redis, &new_info.base).await?;
        Ok(())
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

    async fn update_tank_metrics(
        &self,
        last_battle_time: DateTime,
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
                insert_vehicle_or_ignore(&mut *connection, tank_id).await?;
            }
        }
        Ok(())
    }

    async fn push_sample_points(
        &self,
        account_id: i32,
        tanks: &[Tank],
        opts: &IncrementalOpts,
    ) -> crate::Result {
        let mut points = Vec::new();

        for tank in tanks {
            let tank_id = tank.statistics.base.tank_id;
            let (n_battles, n_wins) =
                retrieve_tank_battle_count(&self.database, account_id, tank_id).await?;
            let n_battles = tank.statistics.all.battles - n_battles;
            let n_wins = tank.statistics.all.wins - n_wins;
            if n_battles > 0 && n_wins >= 0 {
                self.metrics.lock().await.n_battles += n_battles;
                points.push(SamplePoint {
                    account_id,
                    tank_id,
                    n_battles,
                    n_wins,
                    is_test: fastrand::usize(0..100) < opts.test_percentage,
                });
            }
        }

        if !points.is_empty() {
            push_sample_points(&mut self.redis.clone(), &points, opts.training_stream_size).await?;
        }

        Ok(())
    }
}

/// This key contains a mapping from account ID to MessagePack-serialized timestamp in
/// milliseconds. The value is empty string, if the account has never been crawled.
const LAST_BATTLE_TIME_KEY: &str = "last_battle_time::ru";

/// Add the account with empty last battle time, if the account doesn't exist yet.
pub async fn touch_account_if_not_exists(
    redis: &mut MultiplexedConnection,
    account_id: i32,
) -> crate::Result<Option<DateTime>> {
    let (value,): (Option<Vec<u8>>,) = pipe()
        .atomic()
        .hget(LAST_BATTLE_TIME_KEY, account_id)
        .hset_nx(LAST_BATTLE_TIME_KEY, account_id, b"")
        .ignore()
        .query_async(redis)
        .await
        .with_context(|| format!("failed to touch account #{}", account_id))?;
    match value {
        Some(value) if !value.is_empty() => Ok(Some(
            Utc.timestamp_millis(rmp_serde::from_read_ref(&value)?),
        )),
        _ => Ok(None),
    }
}

/// Overwrite the account's last battle time.
async fn set_account_last_battle_time(
    redis: &MultiplexedConnection,
    account: &BaseAccountInfo,
) -> crate::Result {
    let value = rmp_serde::to_vec(&account.last_battle_time.timestamp_millis())?;
    redis
        .clone()
        .hset(LAST_BATTLE_TIME_KEY, account.id, value)
        .await
        .with_context(|| {
            format!(
                "failed to set account #{} last battle time ({:?})",
                account.id, account.last_battle_time,
            )
        })
}
