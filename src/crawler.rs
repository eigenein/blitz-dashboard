use std::collections::HashMap;
use std::sync::Arc;

use futures::{stream, Stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use mongodb::bson::oid::ObjectId;
use tokio::sync::Mutex;

use crate::crawler::crawled_data::CrawledData;
use crate::crawler::metrics::CrawlerMetrics;
use crate::opts::{CrawlAccountsOpts, CrawlerOpts, SharedCrawlerOpts};
use crate::prelude::*;
use crate::wargaming::WargamingApi;
use crate::{database, wargaming};

mod crawled_data;
mod metrics;

pub struct Crawler {
    api: WargamingApi,
    realm: wargaming::Realm,
    db: mongodb::Database,
    metrics: Mutex<CrawlerMetrics>,
    n_buffered_batches: usize,
    heartbeat_url: Option<String>,
    enable_train: bool,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> Result {
    sentry::configure_scope(|scope| {
        scope.set_tag("app", "crawler");
        scope.set_tag("realm", opts.shared.realm);
    });

    let crawler = Crawler::new(&opts.shared, opts.heartbeat_url, opts.enable_train).await?;
    let accounts = database::Account::get_sampled_stream(
        crawler.db.clone(),
        opts.shared.realm,
        opts.sample_size,
        Duration::from_std(opts.min_offset)?,
        opts.offset_scale,
    )?;
    crawler.run(Box::pin(accounts)).await
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
    let crawler = Crawler::new(&opts.shared, None, false).await?;
    crawler.run(accounts).await
}

impl Crawler {
    pub async fn new(
        opts: &SharedCrawlerOpts,
        heartbeat_url: Option<String>,
        enable_train: bool,
    ) -> Result<Self> {
        let api = WargamingApi::new(
            &opts.connections.application_id,
            opts.connections.api_timeout,
            opts.connections.max_api_rps,
        )?;
        let internal = &opts.connections.internal;
        let db = database::mongodb::open(&internal.mongodb_uri).await?;

        let this = Self {
            realm: opts.realm,
            metrics: Mutex::new(CrawlerMetrics::new(
                &api.request_counter,
                opts.lag_percentile,
                opts.lag_window_size,
                opts.log_interval,
            )),
            api,
            db,
            n_buffered_batches: opts.buffering.n_batches,
            heartbeat_url,
            enable_train,
        };
        Ok(this)
    }

    /// Runs the crawler on the stream of batches.
    pub async fn run(
        self,
        accounts: impl Stream<Item = Result<database::Account>> + Unpin,
    ) -> Result {
        info!(realm = ?self.realm, n_buffered_batches = self.n_buffered_batches, "running…");
        let this = Arc::new(self);
        accounts
            .try_chunks(100)
            .map_err(Error::from)
            .try_for_each_concurrent(this.n_buffered_batches, |batch| {
                let this = Arc::clone(&this);
                async move {
                    let mut accounts = this.crawl_batch(batch).await?;
                    while let Some((account, account_info)) = accounts.try_next().await? {
                        let crawled_data = this.crawl_account(account, account_info).await?;
                        let account_id = crawled_data.account.id;
                        this.update_account(crawled_data)
                            .await
                            .with_context(|| anyhow!("failed to update account #{}", account_id))?;
                    }
                    Ok(())
                }
            })
            .await
            .context("the crawler stream has failed")
    }

    #[instrument(skip_all, level = "trace", err)]
    async fn crawl_batch(
        &self,
        batch: Vec<database::Account>,
    ) -> Result<impl Stream<Item = Result<(database::Account, wargaming::AccountInfo)>>> {
        let account_ids: Vec<wargaming::AccountId> =
            batch.iter().map(|account| account.id).collect();
        let new_infos = self.api.get_account_info(self.realm, &account_ids).await?;
        let batch_len = batch.len();
        let matched = Self::match_account_infos(batch, new_infos);

        self.on_batch_crawled(batch_len, matched.len()).await;
        Ok(stream::iter(matched.into_iter()).map(Ok))
    }

    async fn on_batch_crawled(&self, batch_len: usize, matched_len: usize) {
        debug!(matched_len, "batch crawled");

        let mut metrics = self.metrics.lock().await;
        metrics.add_batch(batch_len, matched_len);
        let is_metrics_logged = metrics.check(&self.api.request_counter);
        if let (true, Some(heartbeat_url)) = (is_metrics_logged, &self.heartbeat_url) {
            tokio::spawn(reqwest::get(heartbeat_url.clone()));
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

    /// Crawls account's tank statistics and achievements.
    ///
    /// # Returns
    ///
    /// Updated account, snapshot of the account and snapshots of its tanks.
    #[instrument(
        skip_all,
        level = "debug",
        fields(account_id = account_info.id),
    )]
    async fn crawl_account(
        &self,
        account: database::Account,
        account_info: wargaming::AccountInfo,
    ) -> Result<CrawledData> {
        debug!(?account.last_battle_time);

        let tanks_stats = self
            .api
            .get_tanks_stats(self.realm, account_info.id)
            .await?;
        debug!(n_tanks_stats = tanks_stats.len());
        let tank_last_battle_times = tanks_stats
            .iter()
            .map_into::<database::TankLastBattleTime>()
            .collect_vec();
        let partial_tank_stats = tanks_stats
            .iter()
            .map_into::<database::PartialTankStats>()
            .collect_vec();
        let train_items = if self.enable_train {
            self.gather_train_items(&account, &tanks_stats)
        } else {
            Vec::new()
        };
        let tanks_stats = tanks_stats
            .into_iter()
            .filter(|tank| Some(tank.last_battle_time) > account.last_battle_time)
            .collect_vec();
        let tank_snapshots = if !tanks_stats.is_empty() {
            debug!(n_updated_tanks = tanks_stats.len());
            let achievements = self
                .api
                .get_tanks_achievements(self.realm, account_info.id)
                .await?;
            database::TankSnapshot::from_vec(self.realm, account_info.id, tanks_stats, achievements)
        } else {
            trace!("no updated tanks");
            Vec::new()
        };
        debug!(n_tank_snapshots = tank_snapshots.len(), "crawled");

        let account = database::Account {
            id: account.id,
            realm: self.realm,
            last_battle_time: Some(account_info.last_battle_time),
            partial_tank_stats,
        };
        let account_snapshot =
            database::AccountSnapshot::new(self.realm, &account_info, tank_last_battle_times);
        let rating_snapshot = database::RatingSnapshot::new(self.realm, &account_info);

        Ok(CrawledData {
            account,
            account_snapshot,
            tank_snapshots,
            rating_snapshot,
            train_items,
        })
    }

    /// Gather the recommender system's train data.
    /// Highly experimental.
    #[instrument(level = "debug", skip_all, fields(account_id = account.id))]
    fn gather_train_items(
        &self,
        account: &database::Account,
        actual_tank_stats: &[wargaming::TankStats],
    ) -> Vec<database::TrainItem> {
        let previous_partial_tank_stats = account
            .partial_tank_stats
            .iter()
            .map(|stats| (stats.tank_id, (stats.n_battles, stats.n_wins)))
            .collect::<AHashMap<_, _>>();
        actual_tank_stats
            .iter()
            .filter_map(|stats| {
                previous_partial_tank_stats
                    .get(&stats.tank_id)
                    .and_then(|(n_battles, n_wins)| {
                        let differs = stats.all.n_battles != 0
                            && stats.all.n_battles > *n_battles
                            && stats.all.n_wins >= *n_wins;
                        differs.then(|| database::TrainItem {
                            object_id: ObjectId::from_bytes([0; 12]),
                            realm: account.realm,
                            account_id: account.id,
                            tank_id: stats.tank_id,
                            last_battle_time: stats.last_battle_time,
                            // TODO: check `n_battles >= n_wins`.
                            n_battles: stats.all.n_battles - n_battles,
                            n_wins: stats.all.n_wins - n_wins,
                        })
                    })
            })
            .collect()
    }

    #[instrument(skip_all, fields(account_id = crawled_data.account_snapshot.account_id))]
    async fn update_account(&self, crawled_data: CrawledData) -> Result {
        let start_instant = Instant::now();
        debug!(last_battle_time = ?crawled_data.account.last_battle_time, "updating account…");

        crawled_data.upsert(&self.db).await?;

        let mut metrics = self.metrics.lock().await;
        metrics.add_account(crawled_data.account_snapshot.account_id);
        metrics.add_lag_from(crawled_data.account_snapshot.last_battle_time);
        drop(metrics);

        debug!(elapsed = ?start_instant.elapsed());
        Ok(())
    }
}
