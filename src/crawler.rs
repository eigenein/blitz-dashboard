use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use futures::{stream, Stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::crawler::metrics::CrawlerMetrics;
use crate::helpers::tracing::format_elapsed;
use crate::opts::{BufferingOpts, CrawlAccountsOpts, CrawlerOpts, SharedCrawlerOpts};
use crate::prelude::*;
use crate::wargaming::WargamingApi;
use crate::{database, wargaming};

mod metrics;

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    realm: wargaming::Realm,
    mongodb: mongodb::Database,
    metrics: Arc<Mutex<CrawlerMetrics>>,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let crawler = Crawler::new(&opts.shared).await?;
    let accounts = database::Account::get_sampled_stream(
        crawler.mongodb.clone(),
        opts.shared.realm,
        opts.sample_size,
        Duration::from_std(opts.min_offset)?,
        Duration::from_std(opts.max_offset)?,
    );
    crawler
        .run(accounts, &opts.shared.buffering, opts.heartbeat_url)
        .await
}

/// Performs a very slow one-time account scan.
/// Spawns a crawler which unconditionally inserts and/or updates
/// accounts in the specified range.
///
/// This is a technical script which is intended to be run one time for an entire region
/// to populate the database.
pub async fn crawl_accounts(opts: CrawlAccountsOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawl-accounts"));

    let accounts = stream::iter(opts.start_id..opts.end_id)
        .map(|account_id| database::Account::new(opts.shared.realm, account_id))
        .map(Ok);
    let crawler = Crawler::new(&opts.shared).await?;
    crawler.run(accounts, &opts.shared.buffering, None).await
}

const TIMEOUT: StdDuration = StdDuration::from_secs(60); // FIXME.

impl Crawler {
    pub async fn new(opts: &SharedCrawlerOpts) -> Result<Self> {
        let api = WargamingApi::new(
            &opts.connections.application_id,
            opts.connections.api_timeout,
            opts.connections.max_api_rps,
        )?;
        let internal = &opts.connections.internal;
        let mongodb = database::mongodb::open(&internal.mongodb_uri).await?;

        let this = Self {
            realm: opts.realm,
            metrics: Arc::new(Mutex::new(CrawlerMetrics::new(
                &api.request_counter,
                opts.lag_percentile,
                opts.lag_window_size,
                opts.log_interval,
            ))),
            api,
            mongodb,
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(
        self,
        accounts: impl Stream<Item = Result<database::Account>>,
        buffering: &BufferingOpts,
        heartbeat_url: Option<String>,
    ) -> Result {
        info!(
            realm = ?self.realm,
            n_buffered_batches = buffering.n_batches,
            n_buffered_accounts = buffering.n_buffered_accounts,
            n_updated_accounts = buffering.n_updated_accounts,
            "running…",
        );
        accounts
            .inspect_ok(|account| trace!(account.id, "sampled account"))
            // Chunk in batches of 100 accounts – the maximum for the account information API.
            .try_chunks(100)
            .enumerate()
            // For each batch request basic account information.
            // We need the accounts' last battle timestamps.
            .map(|(batch_number, batch)| {
                let batch = batch?;
                trace!(batch_number, n_accounts = batch.len(), "scheduling the crawler");
                let api = self.api.clone();
                let metrics = self.metrics.clone();
                let heartbeat_url = heartbeat_url.clone();

                Ok(async move {
                    timeout(
                        TIMEOUT,
                        crawl_batch(api, self.realm, batch, batch_number, metrics, heartbeat_url),
                    )
                    .await
                    .with_context(|| format!("timed out to crawl batch #{}", batch_number))?
                })
            })
            // Here we have the stream of batches of accounts that need to be crawled.
            // Now buffer the batches.
            .try_buffer_unordered(buffering.n_batches)
            // Flatten the stream of batches into the stream of non-crawled accounts.
            .try_flatten()
            // Crawl the accounts.
            .map(|item| {
                let (account, account_info) = item?;
                trace!(account.id, "scheduling the crawler");
                let api = self.api.clone();

                Ok(async move {
                    let account_id = account.id;
                    timeout(TIMEOUT, crawl_account(&api, self.realm, account, account_info))
                        .await?
                        .with_context(|| format!("timed out to crawl account #{}", account_id))
                })
            })
            // Buffer the accounts.
            .try_buffer_unordered(buffering.n_buffered_accounts)
            // Make the database updates concurrent.
            .try_for_each_concurrent(
                Some(buffering.n_updated_accounts),
                |(account, account_snapshot, tank_snapshots)| {
                    trace!(
                        account_snapshot.account_id,
                        n_tanks = tank_snapshots.len(),
                        "scheduling the update"
                    );
                    let mongodb = self.mongodb.clone();
                    let metrics = self.metrics.clone();
                    update_account(mongodb, account, account_snapshot, tank_snapshots, metrics)
                },
            )
            .await
            .context("crawler stream has failed")
    }
}

#[instrument(skip_all, level = "debug", fields(batch_number = _batch_number))]
async fn crawl_batch(
    api: WargamingApi,
    realm: wargaming::Realm,
    batch: Vec<database::Account>,
    _batch_number: usize,
    metrics: Arc<Mutex<CrawlerMetrics>>,
    heartbeat_url: Option<String>,
) -> Result<impl Stream<Item = Result<(database::Account, wargaming::AccountInfo)>>> {
    let account_ids: Vec<wargaming::AccountId> = batch.iter().map(|account| account.id).collect();
    let new_infos = api.get_account_info(realm, &account_ids).await?;
    let batch_len = batch.len();
    let matched = match_account_infos(batch, new_infos);

    on_batch_crawled(batch_len, matched.len(), metrics, &api.request_counter, heartbeat_url).await;
    Ok(stream::iter(matched.into_iter()).map(Ok))
}

async fn on_batch_crawled(
    batch_len: usize,
    matched_len: usize,
    metrics: Arc<Mutex<CrawlerMetrics>>,
    request_counter: &AtomicU32,
    heartbeat_url: Option<String>,
) {
    debug!(matched_len, "batch crawled");

    let mut metrics = metrics.lock().await;
    metrics.add_batch(batch_len, matched_len);
    let is_metrics_logged = metrics.check(request_counter);
    if let (true, Some(heartbeat_url)) = (is_metrics_logged, heartbeat_url) {
        tokio::spawn(reqwest::get(heartbeat_url));
    }
}

/// Match the batch's accounts to the account infos fetched from the API.
/// Filters out accounts with unchanged last battle time.
///
/// # Returns
///
/// Vector of matched pairs.
#[instrument(skip_all, level = "debug")]
fn match_account_infos(
    batch: Vec<database::Account>,
    mut new_infos: HashMap<String, Option<wargaming::AccountInfo>>,
) -> Vec<(database::Account, wargaming::AccountInfo)> {
    batch
        .into_iter()
        .filter_map(move |account| match new_infos.remove(&account.id.to_string()).flatten() {
            Some(new_info) if account.last_battle_time != Some(new_info.last_battle_time) => {
                Some((account, new_info))
            }
            _ => None,
        })
        .collect()
}

/// Crawls account from Wargaming.net API, including the tank statistics and achievements.
///
/// # Returns
///
/// Updated account information and account's tanks.
#[instrument(
    skip_all,
    level = "debug",
    fields(account.id = account.id),
)]
async fn crawl_account(
    api: &WargamingApi,
    realm: wargaming::Realm,
    mut account: database::Account,
    account_info: wargaming::AccountInfo,
) -> Result<(database::Account, database::AccountSnapshot, Vec<database::TankSnapshot>)> {
    debug!(?account.last_battle_time);

    let statistics = api.get_tanks_stats(realm, account.id).await?;
    let tank_last_battle_times = statistics
        .iter()
        .map(database::TankLastBattleTime::from)
        .collect_vec();

    let updated_statistics = match account.last_battle_time {
        Some(since) => statistics
            .into_iter()
            .filter(|tank| tank.last_battle_time > since)
            .collect(),
        None => statistics,
    };
    let tank_snapshots = if !updated_statistics.is_empty() {
        debug!(n_updated_tanks = updated_statistics.len());
        let achievements = api.get_tanks_achievements(realm, account.id).await?;
        let tanks = wargaming::merge_tanks(account.id, updated_statistics, achievements);
        debug!(n_tanks = tanks.len(), "crawled");
        tanks
            .into_values()
            .map(|tank| database::TankSnapshot::from_tank(realm, tank))
            .collect()
    } else {
        trace!("no updated tanks");
        Vec::new()
    };

    account.last_battle_time = Some(account_info.last_battle_time);
    let account_snapshot =
        database::AccountSnapshot::new(realm, account_info, tank_last_battle_times);

    Ok((account, account_snapshot, tank_snapshots))
}

#[instrument(skip_all, fields(account_id = account_snapshot.account_id))]
async fn update_account(
    connection: impl Borrow<mongodb::Database>,
    account: database::Account,
    account_snapshot: database::AccountSnapshot,
    tank_snapshots: Vec<database::TankSnapshot>,
    metrics: Arc<Mutex<CrawlerMetrics>>,
) -> Result {
    let connection = connection.borrow();

    debug!(n_tanks = tank_snapshots.len(), "updating account…");
    let start_instant = Instant::now();

    for tank_snapshot in tank_snapshots.into_iter() {
        timeout(TIMEOUT, tank_snapshot.upsert(connection))
            .await
            .context("timed out to upsert the tank snapshot")??;
    }
    debug!(elapsed = format_elapsed(start_instant).as_str(), "tanks upserted");

    timeout(TIMEOUT, account_snapshot.upsert(connection))
        .await
        .context("timed out to upsert the account snapshot")??;
    debug!(elapsed = format_elapsed(start_instant).as_str(), "account snapshot upserted");

    timeout(TIMEOUT, account.upsert(connection, database::Account::OPERATION_SET))
        .await
        .context("timed out to upsert the account")??;
    debug!(elapsed = format_elapsed(start_instant).as_str(), "account upserted");

    let mut metrics = timeout(TIMEOUT, metrics.lock())
        .await
        .context("timed out to lock the metrics")?;
    metrics.add_account(account_snapshot.account_id);
    metrics.add_lag_from(account_snapshot.last_battle_time);
    drop(metrics);

    debug!(elapsed = format_elapsed(start_instant).as_str(), "all done");
    Ok(())
}
