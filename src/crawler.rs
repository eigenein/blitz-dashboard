use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use futures::{stream, Stream, StreamExt, TryStreamExt};
use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::{debug, debug_span, instrument, trace, warn};
use tracing_futures::Instrument;

use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::CrawlerMetrics;
use crate::database::{insert_tank_snapshots, open as open_database, replace_account};
use crate::models::{merge_tanks, Tank};
use crate::opts::{BufferingOpts, CrawlAccountsOpts, CrawlerOpts, SharedCrawlerOpts};
use crate::prelude::*;
use crate::wargaming::models::{AccountInfo, BaseAccountInfo, TankStatistics};
use crate::wargaming::WargamingApi;

pub mod batch_stream;
mod metrics;

const API_TIMEOUT: StdDuration = StdDuration::from_secs(30);

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    metrics: Arc<Mutex<CrawlerMetrics>>,
    log_interval: StdDuration,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let crawler = Crawler::new(&opts.shared).await?;

    warn!("running…");
    let batches = get_batch_stream(
        crawler.database(),
        opts.batch_select_limit,
        opts.min_offset,
        opts.max_offset,
    )
    .await;
    crawler.run(batches, &opts.shared.buffering).await
}

/// Performs a very slow one-time account scan.
/// Spawns a crawler which unconditionally inserts and/or updates
/// accounts in the specified range.
///
/// This is a technical script which is intended to be run one time for an entire region
/// to populate the database.
pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let batches = stream::iter(opts.start_id..opts.end_id)
        .map(BaseAccountInfo::empty)
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(&opts.shared).await?;
    crawler.run(batches, &opts.shared.buffering).await
}

impl Crawler {
    pub async fn new(opts: &SharedCrawlerOpts) -> Result<Self> {
        let api = WargamingApi::new(&opts.connections.application_id, API_TIMEOUT)?;
        let internal = &opts.connections.internal;
        let database = open_database(&internal.database_uri, false).await?;

        let this = Self {
            metrics: Arc::new(Mutex::new(CrawlerMetrics::new(
                &api.request_counter,
                opts.lag_percentile,
            ))),
            api,
            database,
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
        batches: impl Stream<Item = Result<Batch>>,
        buffering: &BufferingOpts,
    ) -> Result {
        batches
            .map_ok(|batch| {
                crawl_batch(self.api.clone(), batch, self.metrics.clone(), self.log_interval)
            })
            .try_buffer_unordered(buffering.n_batches)
            .try_flatten()
            .try_for_each_concurrent(Some(buffering.n_accounts), |(account, new_info, tanks)| {
                self.clone().update_account(account, new_info, tanks)
            })
            .await
            .context("the main crawler stream has failed")
    }

    #[instrument(
        skip_all,
        fields(account_id = account.id, n_tanks = tanks.len()),
    )]
    async fn update_account(
        self,
        account: BaseAccountInfo,
        new_info: AccountInfo,
        tanks: Vec<Tank>,
    ) -> Result {
        let start_instant = Instant::now();
        let mut transaction = self.database.begin().await?;
        insert_tank_snapshots(&mut transaction, &tanks).await?;
        replace_account(&mut transaction, &new_info.base).await?;
        transaction
            .commit()
            .instrument(debug_span!("commit"))
            .await
            .with_context(|| format!("failed to commit account #{}", account.id))?;
        debug!(account_id = account.id, elapsed = ?start_instant.elapsed(), "updated");

        let mut metrics = self.metrics.lock().await;
        metrics.add_account(account.id);
        metrics.add_lag_from(new_info.base.last_battle_time)?;

        Ok(())
    }
}

#[instrument(skip_all, fields(n_accounts = batch.len()))]
async fn crawl_batch(
    api: WargamingApi,
    batch: Batch,
    metrics: Arc<Mutex<CrawlerMetrics>>,
    log_interval: StdDuration,
) -> Result<impl Stream<Item = Result<(BaseAccountInfo, AccountInfo, Vec<Tank>)>>> {
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

    {
        let mut metrics = metrics.lock().await;
        if metrics.start_instant.elapsed() >= log_interval {
            *metrics = metrics.finalise(&api.request_counter).await;
        }
    }

    Ok(stream::iter(crawled.into_iter().map(Ok)))
}

/// Match the batch's accounts to the account infos fetched from the API.
/// Filters out accounts with unchanged last battle time.
///
/// # Returns
///
/// Vector of matched pairs.
#[instrument(skip_all)]
fn match_account_infos(
    batch: Batch,
    mut new_infos: HashMap<String, Option<AccountInfo>>,
) -> Vec<(BaseAccountInfo, AccountInfo)> {
    batch
        .into_iter()
        .filter_map(move |account| match new_infos.remove(&account.id.to_string()).flatten() {
            Some(new_info) if account.last_battle_time != new_info.base.last_battle_time => {
                Some((account, new_info))
            }
            _ => None,
        })
        .collect()
}

/// Gets account tanks which have their last battle time updated since the specified timestamp.
///
/// # Returns
///
/// The tanks statistics as returned by the API.
#[instrument(skip_all, fields(account_id = account_id, since = ?since))]
async fn get_updated_tanks_statistics(
    api: &WargamingApi,
    account_id: i32,
    since: DateTime,
) -> Result<Vec<TankStatistics>> {
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
    skip_all,
    fields(account_id = account.id, last_battle_time = ?account.last_battle_time),
)]
async fn crawl_account(
    api: &WargamingApi,
    account: &BaseAccountInfo,
    new_info: AccountInfo,
) -> Result<(AccountInfo, Vec<Tank>)> {
    let statistics =
        get_updated_tanks_statistics(api, account.id, account.last_battle_time).await?;
    if !statistics.is_empty() {
        debug!(account_id = account.id, n_updated_tanks = statistics.len());
        let achievements = api.get_tanks_achievements(account.id).await?;
        let tanks = merge_tanks(account.id, statistics, achievements);
        debug!(account_id = account.id, n_tanks = tanks.len(), "crawled");
        Ok((new_info, tanks))
    } else {
        trace!(account_id = account.id, "no updated tanks");
        Ok((new_info, Vec::new()))
    }
}
