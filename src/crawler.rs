use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use futures::{stream, Stream, StreamExt, TryStreamExt};
use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing_futures::Instrument;

use crate::crawler::account_stream::get_account_stream;
use crate::crawler::metrics::CrawlerMetrics;
use crate::database::insert_tank_snapshots;
use crate::helpers::tracing::format_elapsed;
use crate::opts::{BufferingOpts, CrawlAccountsOpts, CrawlerOpts, SharedCrawlerOpts};
use crate::prelude::*;
use crate::wargaming::WargamingApi;
use crate::{database, wargaming};

mod account_stream;
mod metrics;

const API_TIMEOUT: StdDuration = StdDuration::from_secs(30);

#[derive(Clone)]
pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
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
    let accounts = get_account_stream(
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
        let api = WargamingApi::new(&opts.connections.application_id, API_TIMEOUT)?;
        let internal = &opts.connections.internal;
        let database = database::open(&internal.database_uri, false).await?;
        let mongodb = database::mongodb::open(&internal.mongodb_uri).await?;

        let this = Self {
            metrics: Arc::new(Mutex::new(CrawlerMetrics::new(
                &api.request_counter,
                opts.lag_percentile,
                opts.lag_window_size,
                opts.log_interval,
            ))),
            api,
            database,
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
            n_buffered_accounts = buffering.n_accounts
        );
        accounts
            .try_chunks(100)
            .map_err(|error| anyhow!(error))
            .instrument(debug_span!("sampled_batch"))
            .enumerate()
            .map(|(batch_number, batch)| {
                Ok(crawl_batch(
                    self.api.clone(),
                    batch?,
                    batch_number,
                    self.metrics.clone(),
                    &heartbeat_url,
                ))
            })
            .try_buffer_unordered(buffering.n_batches)
            .try_flatten()
            .instrument(debug_span!("crawled_account"))
            .try_for_each_concurrent(Some(buffering.n_accounts), |(account, new_info, tanks)| {
                self.clone().update_account(account, new_info, tanks)
            })
            .await
            .context("crawler stream has failed")
    }

    #[instrument(skip_all, fields(account_id = account.id))]
    async fn update_account(
        self,
        account: database::Account,
        new_info: wargaming::AccountInfo,
        tanks: Vec<wargaming::Tank>,
    ) -> Result {
        debug!(n_tanks = tanks.len(), "updating account…");

        let start_instant = Instant::now();
        let mut transaction = self.database.begin().await?;
        insert_tank_snapshots(&mut transaction, &tanks).await?;
        transaction
            .commit()
            .instrument(debug_span!("commit"))
            .await
            .with_context(|| format!("failed to commit account #{}", account.id))?;
        debug!(elapsed = format_elapsed(start_instant).as_str(), "committed to PostgreSQL");

        for tank in tanks.into_iter() {
            database::TankSnapshot::from(tank)
                .upsert(&self.mongodb)
                .await?;
        }
        debug!(elapsed = format_elapsed(start_instant).as_str(), "tanks upserted to MongoDB");

        let last_battle_time = new_info.last_battle_time;
        database::Account::from(new_info)
            .upsert(&self.mongodb, database::Account::OPERATION_SET)
            .await?;
        debug!(elapsed = format_elapsed(start_instant).as_str(), "account upserted to MongoDB");

        let mut metrics = self.metrics.lock().await;
        metrics.add_account(account.id);
        metrics.add_lag_from(last_battle_time)?;
        drop(metrics);

        debug!(elapsed = format_elapsed(start_instant).as_str(), "done");
        Ok(())
    }
}

#[instrument(skip_all, level = "debug", fields(batch_number = _batch_number))]
async fn crawl_batch(
    api: WargamingApi,
    batch: Vec<database::Account>,
    _batch_number: usize,
    metrics: Arc<Mutex<CrawlerMetrics>>,
    heartbeat_url: &Option<String>,
) -> Result<
    impl Stream<Item = Result<(database::Account, wargaming::AccountInfo, Vec<wargaming::Tank>)>>,
> {
    let account_ids: Vec<i32> = batch.iter().map(|account| account.id).collect();
    let new_infos = api.get_account_info(&account_ids).await?;
    let batch_len = batch.len();
    let matched = match_account_infos(batch, new_infos);
    let matched_len = matched.len();
    debug!(len = matched_len, "matched account infos");
    metrics.lock().await.add_batch(batch_len, matched.len());

    let mut crawled = Vec::new();
    for (i, (account, new_info)) in matched.into_iter().enumerate() {
        debug!(i = i + 1, of = matched_len, account.id, last_battle_time = ?account.last_battle_time, "crawling account…");
        let tanks = crawl_account(&api, &account).await?;
        crawled.push((account, new_info, tanks));
    }

    let is_metrics_logged = metrics.lock().await.check(&api.request_counter);
    if let (true, Some(heartbeat_url)) = (is_metrics_logged, heartbeat_url) {
        tokio::spawn(reqwest::get(heartbeat_url.clone()));
    }

    debug!("batch crawled");
    Ok(stream::iter(crawled.into_iter().map(Ok)))
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
    account_id: i32,
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
