use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use chrono::{Duration, Utc};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use humantime::format_duration;
use itertools::Itertools;
use redis::aio::MultiplexedConnection;
use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::instrument;

use crate::battle_stream::entry::{StreamEntry, TankEntry};
use crate::battle_stream::push_entry;
use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::CrawlerMetrics;
use crate::database::{
    insert_tank_snapshots, open as open_database, replace_account,
    retrieve_latest_tank_battle_counts,
};
use crate::models::{
    merge_tanks, AccountInfo, BaseAccountInfo, BattleCounts, Tank, TankStatistics,
};
use crate::opts::{BufferingOpts, CrawlAccountsOpts, CrawlerOpts, SharedCrawlerOpts};
use crate::wargaming::WargamingApi;
use crate::DateTime;

pub mod batch_stream;
mod metrics;

const API_TIMEOUT: StdDuration = StdDuration::from_secs(30);

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    redis: MultiplexedConnection,
    metrics: Arc<Mutex<CrawlerMetrics>>,
    stream_duration: Option<Duration>,
    log_interval: StdDuration,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let crawler = Crawler::new(&opts.shared, Some(opts.stream_duration)).await?;

    tracing::info!("running…");
    let batches =
        get_batch_stream(crawler.database(), opts.batch_select_limit, opts.max_offset).await;
    crawler.run(batches, &opts.shared.buffering).await
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
    let crawler = Crawler::new(&opts.shared, None).await?;
    crawler.run(batches, &opts.shared.buffering).await
}

impl Crawler {
    pub async fn new(
        opts: &SharedCrawlerOpts,
        stream_duration: Option<Duration>,
    ) -> crate::Result<Self> {
        let api = WargamingApi::new(&opts.connections.application_id, API_TIMEOUT)?;
        let internal = &opts.connections.internal;
        let database = open_database(&internal.database_uri, false).await?;
        let redis = redis::Client::open(internal.redis_uri.as_str())?
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect to the Redis")?;

        let this = Self {
            metrics: Arc::new(Mutex::new(CrawlerMetrics::new(
                &api.request_counter,
                opts.lag_percentile,
            ))),
            api,
            database,
            redis,
            stream_duration,
            log_interval: opts.log_interval,
        };
        Ok(this)
    }

    #[must_use]
    pub fn database(&self) -> PgPool {
        self.database.clone()
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(
        self,
        batches: impl Stream<Item = crate::Result<Batch>>,
        buffering: &BufferingOpts,
    ) -> crate::Result {
        batches
            .map_ok(|batch| crawl_batch(self.api.clone(), batch, self.metrics.clone()))
            .try_buffer_unordered(buffering.n_batches)
            .try_flatten()
            .try_for_each_concurrent(Some(buffering.n_accounts), |(account, new_info, tanks)| {
                self.clone().update_account(account, new_info, tanks)
            })
            .await
            .context("the main crawler stream has failed")
    }

    #[tracing::instrument(
        skip_all,
        level = "debug",
        fields(account_id = account.id, n_tanks = tanks.len()),
    )]
    async fn update_account(
        mut self,
        account: BaseAccountInfo,
        new_info: AccountInfo,
        tanks: Vec<Tank>,
    ) -> crate::Result {
        let start_instant = Instant::now();

        if let Some(stream_duration) = self.stream_duration {
            // FIXME: make the `last_battle_time` nullable instead.
            if account.last_battle_time.timestamp() != 0 {
                // Zero timestamp would mean that the account has never played
                // or been crawled before.
                push_incremental_updates(
                    &self.database,
                    &mut self.redis,
                    stream_duration,
                    &self.metrics,
                    account.id,
                    &tanks,
                )
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

        let mut metrics = self.metrics.lock().await;
        metrics.add_account(account.id);
        metrics.add_lag_from(new_info.base.last_battle_time)?;
        if metrics.start_instant.elapsed() >= self.log_interval {
            *metrics = metrics.finalise(&self.api.request_counter).await;
        }

        Ok(())
    }
}

async fn crawl_batch(
    api: WargamingApi,
    batch: Batch,
    metrics: Arc<Mutex<CrawlerMetrics>>,
) -> crate::Result<impl Stream<Item = crate::Result<(BaseAccountInfo, AccountInfo, Vec<Tank>)>>> {
    let account_ids: Vec<i32> = batch.iter().map(|account| account.id).collect();
    let new_infos = api.get_account_info(&account_ids).await?;
    let batch_len = batch.len();
    let matched = match_account_infos(batch, new_infos);
    metrics.lock().await.add_batch(batch_len, matched.len());

    let mut crawled = Vec::new();
    for (account, new_info) in matched.into_iter() {
        let (new_info, tanks) = crawl_account(&api, &account, new_info).await?;
        crawled.push((account, new_info, tanks));
    }
    Ok(stream::iter(crawled.into_iter().map(Ok)))
}

/// When the crawler is being run in the normal incremental mode (and not `crawl-accounts`),
/// it pushes the tanks' differences to the Redis stream, which is used to build
/// the aggregated statistics.
#[instrument(level = "debug", skip_all, fields(account_id = account_id, n_tanks = tanks.len()))]
async fn push_incremental_updates(
    database: &PgPool,
    redis: &mut MultiplexedConnection,
    stream_duration: Duration,
    metrics: &Arc<Mutex<CrawlerMetrics>>,
    account_id: i32,
    tanks: &[Tank],
) -> crate::Result {
    if !tanks.is_empty() {
        let entry =
            prepare_stream_entry(database, metrics, account_id, tanks, stream_duration).await?;
        push_entry(redis, &entry, stream_duration).await?;
    }
    Ok(())
}

/// Converts the account info and tank statistics to the battle stream entry.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(account_id = account_id, n_tanks = tanks.len()),
)]
async fn prepare_stream_entry(
    database: &PgPool,
    metrics: &Arc<Mutex<CrawlerMetrics>>,
    account_id: i32,
    tanks: &[Tank],
    stream_duration: Duration,
) -> crate::Result<StreamEntry> {
    let now = Utc::now();
    let tanks = tanks
        .iter()
        .filter(|tank| {
            // `tanks` contains all tanks that have been played since the last known account battle time.
            // However, some of them may have already become outdated for the stream.
            // Thus, we can optimise the `retrieve_latest_tank_battle_counts` call
            // and also reduce the traffic to Redis.
            now - tank.statistics.base.last_battle_time < stream_duration
        })
        .collect_vec();
    let tank_ids = tanks.iter().map(|tank| tank.tank_id());
    let battle_counts = retrieve_latest_tank_battle_counts(database, account_id, tank_ids).await?;

    let mut entry = StreamEntry {
        account_id,
        tanks: Vec::with_capacity(tanks.len()),
    };
    for tank in tanks {
        let tank_id = tank.tank_id();
        let (n_battles, n_wins) = battle_counts.get(&tank_id).copied().unwrap_or((0, 0));
        let n_battles = tank.statistics.all.battles - n_battles;
        let n_wins = tank.statistics.all.wins - n_wins;
        if n_battles > 0 && n_wins >= 0 {
            metrics.lock().await.n_battles += n_battles;
            entry.tanks.push(TankEntry {
                tank_id,
                timestamp: tank.statistics.base.last_battle_time,
                battle_counts: BattleCounts { n_battles, n_wins },
            });
        }
    }
    Ok(entry)
}

/// Match the batch's accounts to the account infos fetched from the API.
/// Filters out accounts with unchanged last battle time.
///
/// # Returns
///
/// Vector of matched pairs.
fn match_account_infos(
    batch: Batch,
    mut new_infos: HashMap<String, Option<AccountInfo>>,
) -> Vec<(BaseAccountInfo, AccountInfo)> {
    batch
        .into_iter()
        .filter_map(
            move |account| match new_infos.remove(&account.id.to_string()).flatten() {
                Some(new_info) if account.last_battle_time != new_info.base.last_battle_time => {
                    Some((account, new_info))
                }
                _ => None,
            },
        )
        .collect()
}

/// Gets account tanks which have their last battle time updated since the specified timestamp.
///
/// # Returns
///
/// The tanks statistics as returned by the API.
#[instrument(
    level = "debug",
    skip_all,
    fields(account_id = account_id, since = since.to_rfc3339().as_str()),
)]
async fn get_updated_tanks_statistics(
    api: &WargamingApi,
    account_id: i32,
    since: DateTime,
) -> crate::Result<Vec<TankStatistics>> {
    let statistics = api
        .get_tanks_stats(account_id)
        .await?
        .into_iter()
        .filter(|tank| tank.base.last_battle_time > since)
        .collect();
    Ok(statistics)
}

/// Crawls account from Wargaming.net API, including the tank statistics and achievements.
///
/// # Returns
///
/// Updated account information and account's tanks.
#[instrument(
    level = "debug",
    skip_all,
    fields(account_id = account.id, last_battle_time = account.last_battle_time.to_rfc3339().as_str()),
)]
async fn crawl_account(
    api: &WargamingApi,
    account: &BaseAccountInfo,
    new_info: AccountInfo,
) -> crate::Result<(AccountInfo, Vec<Tank>)> {
    let statistics =
        get_updated_tanks_statistics(api, account.id, account.last_battle_time).await?;
    if !statistics.is_empty() {
        tracing::debug!(account_id = account.id, n_updated_tanks = statistics.len());
        let achievements = api.get_tanks_achievements(account.id).await?;
        let tanks = merge_tanks(account.id, statistics, achievements);
        tracing::debug!(account_id = account.id, n_tanks = tanks.len(), "crawled");
        Ok((new_info, tanks))
    } else {
        tracing::trace!(account_id = account.id, "no updated tanks");
        Ok((new_info, Vec::new()))
    }
}
