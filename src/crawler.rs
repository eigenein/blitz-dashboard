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

use crate::cf::{adjust_factors, initialize_factors, predict_win_rate};
use crate::crawler::batch_stream::{get_batch_stream, Batch};
use crate::crawler::metrics::{log_metrics, SubCrawlerMetrics};
use crate::crawler::selector::Selector;
use crate::database;
use crate::database::models::Account;
use crate::database::{retrieve_tank_battle_count, retrieve_tank_ids};
use crate::metrics::Stopwatch;
use crate::models::{merge_tanks, AccountInfo, Tank, TankStatistics};
use crate::opts::{CfOpts, CrawlAccountsOpts, CrawlerOpts};
use crate::trainer::{get_vehicle_factors, set_vehicle_factors, TrainStep};
use crate::wargaming::WargamingApi;

mod batch_stream;
mod metrics;
mod selector;

pub struct Crawler {
    api: WargamingApi,
    database: PgPool,
    redis: Mutex<Redis>,

    n_tasks: usize,
    metrics: Arc<Mutex<SubCrawlerMetrics>>,

    /// `Some(...)` indicates that only tanks with updated last battle time must be crawled.
    /// This also sends out updated tanks to the trainer.
    incremental: Option<IncrementalOpts>,

    /// Used to maintain the vehicle table in the database.
    /// The cache contains tank IDs which are for sure existing at the moment in the database.
    vehicle_cache: Arc<RwLock<HashSet<i32>>>,

    /// Collaborative filtering options.
    cf_opts: CfOpts,
}

pub struct IncrementalOpts {
    #[allow(dead_code)]
    trainer_queue_limit: i32,
}

/// Runs the full-featured account crawler, that infinitely scans all the accounts
/// in the database.
///
/// Intended to be run as a system service.
pub async fn run_crawler(opts: CrawlerOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "crawler"));

    let api = new_wargaming_api(&opts.connections.application_id)?;
    let request_counter = api.request_counter.clone();
    let database = crate::database::open(
        &opts.connections.database_uri,
        opts.connections.initialize_schema,
    )
    .await?;
    let redis = crate::redis::open(&opts.connections.redis_uri).await?;

    let slow_crawler = Crawler::new(
        api.clone(),
        database.clone(),
        redis.clone(),
        1,
        Some(IncrementalOpts {
            trainer_queue_limit: opts.trainer_queue_limit,
        }),
        opts.cf,
    )
    .await?;
    let fast_crawler = Crawler::new(
        api,
        database.clone(),
        redis,
        opts.n_fast_tasks,
        Some(IncrementalOpts {
            trainer_queue_limit: opts.trainer_queue_limit,
        }),
        opts.cf,
    )
    .await?;
    let metrics = vec![fast_crawler.metrics.clone(), slow_crawler.metrics.clone()];

    log::info!("Running…");
    tokio::spawn(log_metrics(request_counter, metrics, opts.log_interval));
    let fast_run = fast_crawler.run(get_batch_stream(
        database.clone(),
        Selector::Between(opts.min_offset, opts.slow_offset),
    ));
    let slow_run = slow_crawler.run(get_batch_stream(
        database,
        Selector::Before(opts.slow_offset),
    ));

    futures::future::try_join(fast_run, slow_run).await?;
    Ok(())
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
    let redis = crate::redis::open(&opts.connections.redis_uri).await?;

    let stream = stream::iter(opts.start_id..opts.end_id)
        .map(Account::empty)
        .chunks(100)
        .map(Ok);
    let crawler = Crawler::new(api.clone(), database, redis, opts.n_tasks, None, opts.cf).await?;
    tokio::spawn(log_metrics(
        api.request_counter.clone(),
        vec![crawler.metrics.clone()],
        StdDuration::from_secs(60),
    ));
    crawler.run(stream).await?;
    Ok(())
}

fn new_wargaming_api(application_id: &str) -> crate::Result<WargamingApi> {
    WargamingApi::new(application_id, StdDuration::from_millis(3000))
}

impl Crawler {
    pub async fn new(
        api: WargamingApi,
        database: PgPool,
        redis: Redis,
        n_tasks: usize,
        incremental: Option<IncrementalOpts>,
        cf_opts: CfOpts,
    ) -> crate::Result<Self> {
        let tank_ids: HashSet<i32> = retrieve_tank_ids(&database).await?.into_iter().collect();
        let this = Self {
            api,
            database,
            redis: Mutex::new(redis),
            n_tasks,
            incremental,
            cf_opts,
            metrics: Arc::new(Mutex::new(SubCrawlerMetrics::default())),
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
        let account_ids: Vec<i32> = batch.iter().map(|account| account.base.id).collect();
        let mut new_infos = self.api.get_account_info(&account_ids).await?;

        let mut tx = self.database.begin().await?;
        for account in batch.into_iter() {
            let account_id = account.base.id;
            if let Some(new_info) = new_infos.remove(&account_id.to_string()).flatten() {
                self.crawl_account(&mut tx, account, new_info).await?;
            }
            self.update_metrics_for_account(account_id).await;
        }
        log::debug!("Committing…");
        tx.commit().await.with_context(|| {
            let first_id = account_ids.first();
            let last_id = account_ids.last();
            format!("failed to commit the batch {:?}..{:?}", first_id, last_id)
        })?;

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
        new_info: AccountInfo,
    ) -> crate::Result {
        let _stopwatch = Stopwatch::new(format!("Account #{} crawled", account.base.id));

        if new_info.base.last_battle_time == account.base.last_battle_time {
            log::trace!("#{}: last battle time is not changed.", account.base.id);
            return Ok(());
        }

        let base = account.base;
        let mut account_factors = account.factors;
        log::debug!("Crawling account #{}…", base.id);
        let statistics = self
            .get_updated_tanks_statistics(base.id, base.last_battle_time)
            .await?;
        if !statistics.is_empty() {
            let achievements = self.api.get_tanks_achievements(base.id).await?;
            let tanks = merge_tanks(base.id, statistics, achievements);
            if self.incremental.is_some() {
                self.train(base.id, &mut account_factors, &tanks).await?;
            }
            database::insert_tank_snapshots(&mut *connection, &tanks).await?;
            self.insert_missing_vehicles(&mut *connection, &tanks)
                .await?;

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
                factors: account_factors,
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
        account_factors: &mut Vec<f64>,
        tanks: &[Tank],
    ) -> crate::Result {
        let mut steps = Vec::new();

        for tank in tanks {
            let tank_id = tank.statistics.base.tank_id;
            let (n_battles, n_wins) =
                retrieve_tank_battle_count(&self.database, account_id, tank_id).await?;
            let n_battles = tank.statistics.all.battles - n_battles;
            let n_wins = tank.statistics.all.wins - n_wins;
            if n_battles > 0 && n_wins >= 0 {
                for i in 0..n_battles {
                    steps.push(TrainStep {
                        account_id,
                        tank_id,
                        is_win: i < n_wins,
                    });
                }
            }
        }

        fastrand::shuffle(&mut steps); // make it even more stochastic
        initialize_factors(account_factors, self.cf_opts.n_factors);

        for step in steps {
            let mut redis = self.redis.lock().await;
            let mut vehicle_factors = get_vehicle_factors(&mut redis, step.tank_id).await?;
            initialize_factors(&mut vehicle_factors, self.cf_opts.n_factors);
            let prediction = predict_win_rate(&vehicle_factors, account_factors);
            let target = if step.is_win { 1.0 } else { 0.0 };
            self.metrics.lock().await.push_error(prediction, target);
            self.make_train_step(account_factors, &mut vehicle_factors, prediction, target);
            set_vehicle_factors(&mut redis, step.tank_id, &vehicle_factors).await?;
        }

        Ok(())
    }

    fn make_train_step(
        &self,
        account_factors: &mut [f64],
        vehicle_factors: &mut [f64],
        prediction: f64,
        target: f64,
    ) {
        let residual_error = target - prediction;

        // Adjust the latent factors.
        let frozen_account_factors = account_factors.to_vec();
        adjust_factors(
            account_factors,
            vehicle_factors,
            residual_error,
            self.cf_opts.account_learning_rate,
            self.cf_opts.r,
        );
        adjust_factors(
            vehicle_factors,
            &frozen_account_factors,
            residual_error,
            self.cf_opts.vehicle_learning_rate,
            self.cf_opts.r,
        );
    }
}
