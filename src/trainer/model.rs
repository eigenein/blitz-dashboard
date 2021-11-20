use anyhow::Context;
use std::cmp::Ordering;
use std::time::Instant;

use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use lru::LruCache;
use rand::prelude::Distribution;
use rand::thread_rng;
use rand_distr::Normal;
use redis::aio::MultiplexedConnection;
use redis::{pipe, AsyncCommands};

use crate::helpers::format_elapsed;
use crate::opts::TrainerModelOpts;
use crate::Vector;

const VEHICLE_FACTORS_KEY: &str = "cf::vehicles";

pub async fn get_account_factors(
    redis: &mut MultiplexedConnection,
    account_id: i32,
) -> crate::Result<Option<Vector>> {
    let bytes: Option<Vec<u8>> = redis.get(format!("f::ru::{}", account_id)).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

pub async fn get_vehicle_factors(
    redis: &mut MultiplexedConnection,
    tank_id: i32,
) -> crate::Result<Option<Vector>> {
    let bytes: Option<Vec<u8>> = redis.hget(VEHICLE_FACTORS_KEY, tank_id).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

pub async fn get_all_vehicle_factors(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<i32, Vector>> {
    let hash_map: std::collections::HashMap<i32, Vec<u8>> =
        redis.hgetall(VEHICLE_FACTORS_KEY).await?;
    hash_map
        .into_iter()
        .map(|(tank_id, value)| Ok((tank_id, rmp_serde::from_read_ref(&value)?)))
        .collect()
}

pub struct Model {
    pub n_new_accounts: usize,
    pub n_initialized_accounts: usize,
    pub opts: TrainerModelOpts,

    redis: Option<MultiplexedConnection>,
    vehicle_cache: HashMap<i32, Vector>,
    account_cache: LruCache<i32, Vector>,
    modified_account_ids: HashSet<i32>,
    last_flush_instant: Instant,
}

pub struct Factors<'a> {
    pub account: &'a mut Vector,
    pub vehicle: &'a mut Vector,
}

impl Model {
    pub fn new(redis: Option<MultiplexedConnection>, opts: TrainerModelOpts) -> Self {
        Self {
            redis,
            opts,
            vehicle_cache: HashMap::new(),
            account_cache: LruCache::unbounded(),
            modified_account_ids: HashSet::new(),
            last_flush_instant: Instant::now(),
            n_new_accounts: 0,
            n_initialized_accounts: 0,
        }
    }

    pub fn n_modified_accounts(&self) -> usize {
        self.modified_account_ids.len()
    }

    pub async fn get_factors_mut(
        &mut self,
        account_id: i32,
        tank_id: i32,
    ) -> crate::Result<Factors<'_>> {
        if !self.account_cache.contains(&account_id) {
            let factors = if let Some(redis) = &mut self.redis {
                get_account_factors(redis, account_id).await?
            } else {
                None
            };
            let mut factors = factors.unwrap_or_else(|| {
                self.n_new_accounts += 1;
                Vector::new()
            });
            if initialize_factors(&mut factors, self.opts.n_factors, self.opts.factor_std) {
                self.n_initialized_accounts += 1;
            }
            self.account_cache.put(account_id, factors);
        };
        let account = self.account_cache.get_mut(&account_id).unwrap();

        let vehicle = match self.vehicle_cache.entry(tank_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let factors = if let Some(redis) = &mut self.redis {
                    get_vehicle_factors(redis, tank_id).await?
                } else {
                    None
                };
                let mut factors = factors.unwrap_or_else(Vector::new);
                initialize_factors(&mut factors, self.opts.n_factors, self.opts.factor_std);
                entry.insert(factors)
            }
        };

        Ok(Factors { account, vehicle })
    }

    /// Marks the account factors as modified.
    pub fn touch(&mut self, account_id: i32) {
        self.modified_account_ids.insert(account_id);
    }

    #[tracing::instrument(skip_all)]
    pub async fn flush(&mut self) -> crate::Result {
        if self.last_flush_instant.elapsed() >= self.opts.commit_period {
            self.force_flush().await?;
            self.last_flush_instant = Instant::now();
        }
        Ok(())
    }

    /// Store all the account and vehicle factors to Redis and shrink the caches.
    #[tracing::instrument(skip_all)]
    async fn force_flush(&mut self) -> crate::Result {
        if let Some(redis) = &mut self.redis {
            tracing::info!(n_accounts = self.modified_account_ids.len(), "flushing…");
            let start_instant = Instant::now();
            let mut pipeline = pipe();
            for account_id in self.modified_account_ids.drain() {
                let bytes = rmp_serde::to_vec(self.account_cache.peek(&account_id).unwrap())?;
                let key = format!("f::ru::{}", account_id);
                pipeline
                    .set_ex(key, bytes, self.opts.account_ttl_secs)
                    .ignore();
            }
            self.account_cache.resize(self.opts.account_cache_size);
            let vehicles: crate::Result<Vec<(i32, Vec<u8>)>> = self
                .vehicle_cache
                .iter()
                .map(|(tank_id, factors)| Ok((*tank_id, rmp_serde::to_vec(factors)?)))
                .collect();
            pipeline
                .hset_multiple(VEHICLE_FACTORS_KEY, &vehicles?)
                .ignore()
                .query_async(redis)
                .await
                .context("failed to flush the factors")?;
            tracing::info!(
                elapsed = format_elapsed(&start_instant).as_str(),
                "factors flushed",
            );
        }

        Ok(())
    }
}

fn initialize_factors(x: &mut Vector, n: usize, factor_std: f64) -> bool {
    match x.len().cmp(&n) {
        Ordering::Equal => false,
        _ => {
            let mut rng = thread_rng();
            let distribution = Normal::new(0.0, factor_std).unwrap();
            x.clear();
            while x.len() < n {
                x.push(distribution.sample(&mut rng));
            }
            true
        }
    }
}
