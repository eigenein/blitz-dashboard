use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use chrono::{Duration, Utc};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};
use humantime::format_duration;
use itertools::Itertools;
use redis::aio::MultiplexedConnection;
use redis::pipe;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::CrawlerMetrics;
use crate::database::{
    insert_tank_snapshots, open as open_database, replace_account,
    retrieve_latest_tank_battle_counts,
};
use crate::models::{merge_tanks, AccountInfo, BaseAccountInfo, Tank, TankStatistics};
use crate::opts::{BufferingOpts, CrawlAccountsOpts, CrawlerOpts, SharedCrawlerOpts};
use crate::trainer::dataset::push_stream_entries;
use crate::trainer::stream_entry::StreamEntry;
use crate::wargaming::WargamingApi;
use crate::DateTime;

pub mod batch_stream;
mod metrics;

const API_TIMEOUT: StdDuration = StdDuration::from_secs(30);

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    redis: MultiplexedConnection,

    buffering: BufferingOpts,

    metrics: CrawlerMetrics,
    auto_min_offset: Option<Arc<RwLock<StdDuration>>>,

    /// `Some(...)` indicates that only tanks with updated last battle time must be crawled.
    /// This also sends out updated tanks to the trainer.
    incremental: Option<IncrementalOpts>,
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

    let min_offset = Arc::new(RwLock::new(opts.min_offset));
    let crawler = Crawler::new(
        opts.shared,
        Some(IncrementalOpts {
            training_stream_size: opts.training_stream_size,
            training_stream_duration: opts.training_stream_duration,
            test_percentage: opts.test_percentage,
        }),
        opts.auto_min_offset.then(|| min_offset.clone()),
    )
    .await?;

    tracing::info!("running…");
    let batches = get_batch_stream(crawler.database(), crawler.redis(), min_offset).await;
    crawler.run(Box::pin(batches)).await
}

/// Performs a very slow one-time account scan.
/// Spawns a crawler which unconditionally inserts and/or updates
/// accounts in the specified range.
///
/// This is a technical script which is intended to be run one time for an entire region
/// to populate the database.
pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let batches = stream::iter(opts.start_id..opts.end_id)
        .map(BaseAccountInfo::empty)
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(opts.shared, None, None).await?;
    crawler.run(batches).await
}

impl Crawler {
    pub async fn new(
        opts: SharedCrawlerOpts,
        incremental: Option<IncrementalOpts>,
        auto_min_offset: Option<Arc<RwLock<StdDuration>>>,
    ) -> crate::Result<Self> {
        let api = WargamingApi::new(
            &opts.connections.application_id,
            API_TIMEOUT,
            Some(opts.throttling_period),
        )?;
        let internal = opts.connections.internal;
        let database = open_database(&internal.database_uri, internal.initialize_schema).await?;
        let redis = redis::Client::open(internal.redis_uri.as_str())?
            .get_multiplexed_async_connection()
            .await?;

        let this = Self {
            metrics: CrawlerMetrics::new(api.request_counter.clone(), opts.log_interval),
            api,
            database,
            redis,
            buffering: opts.buffering,
            incremental,
            auto_min_offset,
        };
        Ok(this)
    }

    #[must_use]
    pub fn database(&self) -> PgPool {
        self.database.clone()
    }

    #[must_use]
    pub fn redis(&self) -> MultiplexedConnection {
        self.redis.clone()
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(
        mut self,
        batches: impl Stream<Item = crate::Result<Batch>> + Unpin,
    ) -> crate::Result {
        let api = self.api.clone();

        let accounts = batches
            // Get account info for all accounts in the batch.
            .map_ok(|batch| async {
                let account_ids: Vec<i32> = batch.iter().map(|account| account.id).collect();
                let new_infos = api.get_account_info(&account_ids).await?;
                Ok((batch, new_infos))
            })
            // Parallelize `get_account_info`.
            .try_buffer_unordered(self.buffering.n_batches)
            // Match the retrieved infos against the accounts from the batch.
            .and_then(|(batch, new_infos)| async { Ok(zip_account_infos(batch, new_infos)) })
            // Convert them to the stream of account infos.
            .try_flatten()
            // Crawl the accounts.
            .map_ok(|(account, new_info)| crawl_account(&api, account, new_info))
            // Parallelize `crawl_account`.
            .try_buffer_unordered(self.buffering.n_accounts)
            // Filter out unchanged accounts.
            .try_filter_map(|item| future::ready(Ok(item)));

        // Update the changed accounts in the database.
        let mut accounts = Box::pin(accounts);
        while let Some((account, new_info, tanks)) = accounts.try_next().await? {
            self.metrics.add_account(account.id);
            self.metrics
                .add_tanks(new_info.base.last_battle_time, tanks.len())?;
            self.update_account(account, new_info, tanks).await?;
            self.metrics.check(&self.auto_min_offset).await;
        }

        Ok(())
    }

    #[tracing::instrument(
        skip_all,
        level = "debug",
        fields(account_id = account.id, n_tanks = tanks.len()),
    )]
    async fn update_account(
        &mut self,
        account: BaseAccountInfo,
        new_info: AccountInfo,
        tanks: Vec<Tank>,
    ) -> crate::Result {
        let start_instant = Instant::now();

        if let Some(opts) = self.incremental {
            // FIXME: make the `last_battle_time` nullable instead.
            if account.last_battle_time.timestamp() != 0 {
                // Zero timestamp would mean that the account has never played
                // or been crawled before.
                self.push_incremental_updates(&opts, account.id, &tanks)
                    .await?;
            }
        }

        let mut transaction = self.database.begin().await?;
        insert_tank_snapshots(&mut transaction, &tanks).await?;
        replace_account(&mut transaction, &new_info.base).await?;
        transaction
            .commit()
            .await
            .with_context(|| format!("failed to commit account #{}", account.id))?;

        tracing::debug!(account_id = account.id, elapsed = %format_duration(start_instant.elapsed()), "updated");
        Ok(())
    }

    /// Prepares the training stream entries.
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
        let battle_counts = retrieve_latest_tank_battle_counts(
            &self.database,
            account_id,
            &tanks.iter().map(Tank::tank_id).collect_vec(),
        )
        .await?;

        let now = Utc::now();
        let mut entries = Vec::new();

        for tank in tanks {
            let last_battle_time = tank.statistics.base.last_battle_time;
            if now - last_battle_time > opts.training_stream_duration {
                tracing::debug!(tank_id = tank.tank_id(), "the last battle is too old");
                continue;
            }
            let tank_id = tank.tank_id();
            let (n_battles, n_wins) = battle_counts.get(&tank_id).copied().unwrap_or((0, 0));
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

    #[instrument(level = "debug", skip_all, fields(account_id = account_id, n_tanks = tanks.len()))]
    async fn push_incremental_updates(
        &mut self,
        opts: &IncrementalOpts,
        account_id: i32,
        tanks: &[Tank],
    ) -> crate::Result {
        if tanks.is_empty() {
            return Ok(());
        }

        let entries = self.prepare_stream_entries(account_id, tanks, opts).await?;
        push_stream_entries(
            &mut self.redis,
            &entries,
            opts.training_stream_size,
            opts.training_stream_duration,
        )
        .await?;
        self.push_vehicle_analytics(tanks).await?;

        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    async fn push_vehicle_analytics(&mut self, tanks: &[Tank]) -> crate::Result {
        if tanks.is_empty() {
            return Ok(());
        }

        const TTL_SECS: usize = 7 * 24 * 60 * 60;

        let mut pipeline = pipe();
        for tank in tanks {
            let (n_battles_key, n_wins_key) =
                make_analytics_keys(tank.statistics.base.last_battle_time);
            let tank_id = tank.statistics.base.tank_id;
            pipeline
                .hincr(&n_battles_key, tank_id, tank.statistics.all.battles)
                .ignore()
                .expire(&n_battles_key, TTL_SECS)
                .ignore()
                .hincr(&n_wins_key, tank_id, tank.statistics.all.wins)
                .ignore()
                .expire(&n_wins_key, TTL_SECS)
                .ignore();
        }
        pipeline
            .query_async(&mut self.redis)
            .await
            .context("failed to push vehicle analytics")
    }
}

#[must_use]
pub fn make_analytics_keys(timestamp: DateTime) -> (String, String) {
    let infix = timestamp.format("%Y%m%d%H");
    let n_battles_key = format!("analytics::ru::{}::n_battles", infix);
    let n_wins_key = format!("analytics::ru::{}::n_wins", infix);
    (n_battles_key, n_wins_key)
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

/// Gets account tanks which have their last battle time updated since the specified timestamp.
async fn get_updated_tanks_statistics(
    api: &WargamingApi,
    account_id: i32,
    since: DateTime,
) -> crate::Result<Vec<TankStatistics>> {
    Ok(api
        .get_tanks_stats(account_id)
        .await?
        .into_iter()
        .filter(|tank| tank.base.last_battle_time > since)
        .collect())
}

#[instrument(
    level = "debug",
    skip_all,
    fields(account_id = account.id, since = account.last_battle_time.to_rfc3339().as_str()),
)]
async fn crawl_account(
    api: &WargamingApi,
    account: BaseAccountInfo,
    new_info: AccountInfo,
) -> crate::Result<Option<(BaseAccountInfo, AccountInfo, Vec<Tank>)>> {
    if new_info.base.last_battle_time == account.last_battle_time {
        tracing::trace!(account_id = account.id, "last battle time unchanged");
        return Ok(None);
    }
    let statistics =
        get_updated_tanks_statistics(api, account.id, account.last_battle_time).await?;
    if !statistics.is_empty() {
        tracing::debug!(account_id = account.id, n_updated_tanks = statistics.len());
        let achievements = api.get_tanks_achievements(account.id).await?;
        let tanks = merge_tanks(account.id, statistics, achievements);
        tracing::debug!(account_id = account.id, n_tanks = tanks.len(), "crawled");
        Ok(Some((account, new_info, tanks)))
    } else {
        tracing::trace!(account_id = account.id, "no updated tanks");
        Ok(Some((account, new_info, Vec::new())))
    }
}
