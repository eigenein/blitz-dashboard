use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Error};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use tokio::sync::Mutex;

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

    info!("running…");
    let accounts = database::Account::get_sampled_stream(
        crawler.mongodb.clone(),
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
        .map(database::Account::fake)
        .map(Ok);
    let crawler = Crawler::new(&opts.shared).await?;
    crawler.run(accounts, &opts.shared.buffering, None).await
}

impl Crawler {
    pub async fn new(opts: &SharedCrawlerOpts) -> Result<Self> {
        let api = WargamingApi::new(&opts.connections.application_id, opts.api_timeout)?;
        let internal = &opts.connections.internal;
        let mongodb = database::mongodb::open(&internal.mongodb_uri).await?;

        let this = Self {
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
            n_buffered_batches = buffering.n_batches,
            n_buffered_accounts = buffering.n_buffered_accounts,
            n_updated_accounts = buffering.n_updated_accounts,
        );
        accounts
            .inspect_ok(|account| trace!(account.id, "sampled account (crawler)"))
            // Chunk in batches of 100 accounts – the maximum for the account information API.
            .try_chunks(100)
            .map_err(Error::from)
            .enumerate()
            .inspect(|(batch_number, batch)| trace!(batch_number, is_ok = batch.is_ok(), "sampled batch"))
            // For each batch request basic account information.
            // We need the accounts' last battle timestamps.
            .map(|(batch_number, batch)| Ok(crawl_batch(
                self.api.clone(),
                batch?,
                batch_number,
                self.metrics.clone(),
            )))
            // Here we have the stream of batches of accounts that need to be crawled.
            // Now buffer the batches.
            .try_buffer_unordered(buffering.n_batches)
            // Flatten the stream of batches into the stream of non-crawled accounts.
            .try_flatten()
            .inspect_ok(|(account, _account_info)| trace!(account.id, "account is about to get crawled"))
            // Crawl the accounts.
            .map(|item| {
                let (account, account_info) = item?;
                let api = self.api.clone();
                let heartbeat_url = heartbeat_url.clone();
                let metrics = self.metrics.clone();

                Ok(async move {
                    // TODO: extract the metrics tracing.
                    let is_metrics_logged = metrics.lock().await.check(&api.request_counter);
                    if let (true, Some(heartbeat_url)) = (is_metrics_logged, heartbeat_url) {
                        tokio::spawn(reqwest::get(heartbeat_url));
                    }

                    debug!(account.id, last_battle_time = ?account.last_battle_time, "crawling account…");
                    let tanks = crawl_account(&api, &account).await?;
                    Ok((account, account_info, tanks))
                })
            })
            // Buffer the accounts.
            .try_buffer_unordered(buffering.n_buffered_accounts)
            .inspect_ok(|(account, _account_info, tanks)| trace!(account.id, n_tanks = tanks.len(), "crawled account"))
            // Make the database updates concurrent.
            .try_for_each_concurrent(
                Some(buffering.n_updated_accounts),
                |(account, new_info, tanks)| update_account(self.mongodb.clone(), account, new_info, tanks, self.metrics.clone()),
            )
            .await
            .context("crawler stream has failed")
    }
}

#[instrument(skip_all, level = "debug", fields(batch_number = _batch_number))]
async fn crawl_batch(
    api: WargamingApi,
    batch: Vec<database::Account>,
    _batch_number: usize,
    metrics: Arc<Mutex<CrawlerMetrics>>,
) -> Result<impl Stream<Item = Result<(database::Account, wargaming::AccountInfo)>>> {
    let account_ids: Vec<wargaming::AccountId> = batch.iter().map(|account| account.id).collect();
    let new_infos = api.get_account_info(&account_ids).await?;
    let batch_len = batch.len();
    let matched = match_account_infos(batch, new_infos);

    debug!(matched_len = matched.len(), "batch crawled");
    metrics.lock().await.add_batch(batch_len, matched.len());

    Ok(stream::iter(matched.into_iter()).map(Ok))
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

/// Gets account tanks which have their last battle time updated since the specified timestamp.
///
/// # Returns
///
/// The tanks statistics as returned by the API.
#[instrument(skip_all, fields(account_id = account_id, since = ?since))]
async fn get_updated_tanks_statistics(
    api: &WargamingApi,
    account_id: wargaming::AccountId,
    since: Option<DateTime>,
) -> Result<Vec<wargaming::TankStatistics>> {
    let statistics = api.get_tanks_stats(account_id).await?;
    let statistics = match since {
        Some(since) => statistics
            .into_iter()
            .filter(|tank| tank.basic.last_battle_time > since)
            .collect(),
        None => statistics,
    };
    Ok(statistics)
}

/// Crawls account from Wargaming.net API, including the tank statistics and achievements.
///
/// # Returns
///
/// Updated account information and account's tanks.
#[instrument(
    skip_all,
    level = "debug",
    fields(account_id = account.id),
)]
async fn crawl_account(
    api: &WargamingApi,
    account: &database::Account,
) -> Result<Vec<wargaming::Tank>> {
    let statistics =
        get_updated_tanks_statistics(api, account.id, account.last_battle_time).await?;
    if !statistics.is_empty() {
        debug!(n_updated_tanks = statistics.len());
        let achievements = api.get_tanks_achievements(account.id).await?;
        let tanks = wargaming::merge_tanks(account.id, statistics, achievements);
        debug!(n_tanks = tanks.len(), "crawled");
        Ok(tanks)
    } else {
        trace!("no updated tanks");
        Ok(Vec::new())
    }
}

#[instrument(skip_all, fields(account_id = account.id))]
async fn update_account(
    connection: impl Borrow<mongodb::Database>,
    account: database::Account,
    new_info: wargaming::AccountInfo,
    tanks: Vec<wargaming::Tank>,
    metrics: Arc<Mutex<CrawlerMetrics>>,
) -> Result {
    let connection = connection.borrow();

    debug!(n_tanks = tanks.len(), "updating account…");
    let start_instant = Instant::now();

    for tank in tanks.into_iter() {
        database::TankSnapshot::from(tank)
            .upsert(connection)
            .await?;
    }
    debug!(elapsed = format_elapsed(start_instant).as_str(), "tanks upserted to MongoDB");

    let last_battle_time = new_info.last_battle_time;
    database::Account::from(new_info)
        .upsert(connection, database::Account::OPERATION_SET)
        .await?;
    debug!(elapsed = format_elapsed(start_instant).as_str(), "account upserted to MongoDB");

    let mut metrics = metrics.lock().await;
    metrics.add_account(account.id);
    metrics.add_lag_from(last_battle_time);
    drop(metrics);

    debug!(elapsed = format_elapsed(start_instant).as_str(), "all done");
    Ok(())
}
