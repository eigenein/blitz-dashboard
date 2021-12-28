use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::time::Instant;

use anyhow::Context;
use itertools::Itertools;
use rand::prelude::Distribution;
use rand::thread_rng;
use rand_distr::Normal;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tracing::{info, instrument};

use crate::helpers::format_elapsed;
use crate::helpers::periodic::Periodic;
use crate::math::vector::Vector;
use crate::opts::TrainerModelOpts;
use crate::wargaming::tank_id::TankId;

const VEHICLE_FACTORS_KEY: &str = "trainer::vehicles";
const ACCOUNT_FACTORS_KEY: &str = "trainer::accounts::ru";
const REGULARIZATION_KEY: &str = "trainer::r";

type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
type HashSet<V> = std::collections::HashSet<V, ahash::RandomState>;
type LruCache<K, V> = lru::LruCache<K, V, ahash::RandomState>;

pub async fn get_account_factors(
    redis: &mut MultiplexedConnection,
    account_id: i32,
) -> crate::Result<Option<Vector>> {
    let bytes: Option<Vec<u8>> = redis.hget(ACCOUNT_FACTORS_KEY, account_id).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

pub async fn get_vehicle_factors(
    redis: &mut MultiplexedConnection,
    tank_id: u16,
) -> crate::Result<Option<Vector>> {
    let bytes: Option<Vec<u8>> = redis.hget(VEHICLE_FACTORS_KEY, tank_id).await?;
    match bytes {
        Some(bytes) => Ok(rmp_serde::from_read_ref(&bytes)?),
        None => Ok(None),
    }
}

pub async fn get_all_vehicle_factors(
    redis: &mut MultiplexedConnection,
) -> crate::Result<HashMap<TankId, Vector>> {
    let hash_map: std::collections::HashMap<u16, Vec<u8>> =
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
    pub regularization: f64,

    redis: MultiplexedConnection,
    vehicle_cache: HashMap<u16, Vector>,
    account_cache: LruCache<i32, Vector>,
    modified_account_ids: HashSet<i32>,
    periodic_flush: Periodic,
}

pub struct Factors<'a> {
    pub account: &'a mut Vector,
    pub vehicle: &'a mut Vector,
}

impl Model {
    pub async fn new(
        mut redis: MultiplexedConnection,
        opts: TrainerModelOpts,
    ) -> crate::Result<Self> {
        let regularization = get_regularization(&mut redis).await?;
        Ok(Self {
            redis,
            opts,
            regularization,
            vehicle_cache: HashMap::default(),
            account_cache: LruCache::unbounded_with_hasher(ahash::RandomState::default()),
            modified_account_ids: HashSet::default(),
            n_new_accounts: 0,
            n_initialized_accounts: 0,
            periodic_flush: Periodic::new(opts.flush_interval),
        })
    }

    pub fn n_modified_accounts(&self) -> usize {
        self.modified_account_ids.len()
    }

    pub async fn get_factors_mut(
        &mut self,
        account_id: i32,
        tank_id: u16,
    ) -> crate::Result<Factors<'_>> {
        if !self.account_cache.contains(&account_id) {
            let factors = get_account_factors(&mut self.redis, account_id).await?;
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
                let factors = get_vehicle_factors(&mut self.redis, tank_id).await?;
                let mut factors = factors.unwrap_or_else(Vector::new);
                initialize_factors(&mut factors, self.opts.n_factors, self.opts.factor_std);
                entry.insert(factors)
            }
        };

        Ok(Factors { account, vehicle })
    }

    /// Marks the account factors as modified.
    pub fn touch_account(&mut self, account_id: i32) {
        self.modified_account_ids.insert(account_id);
    }

    #[instrument(skip_all)]
    pub async fn flush(&mut self) -> crate::Result {
        if self.periodic_flush.should_trigger() {
            self.force_flush().await?;
        }
        Ok(())
    }

    /// Store all the account and vehicle factors to Redis and shrink the caches.
    #[instrument(skip_all)]
    async fn force_flush(&mut self) -> crate::Result {
        let start_instant = Instant::now();
        self.force_flush_accounts().await?;
        self.force_flush_vehicles().await?;
        set_regularization(&mut self.redis, self.regularization).await?;
        info!(
            elapsed = %format_elapsed(&start_instant),
            "model flushed",
        );
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn force_flush_accounts(&mut self) -> crate::Result {
        const BATCH_SIZE: usize = 1_000_000;
        info!(
            n_accounts = self.modified_account_ids.len(),
            batch_size = BATCH_SIZE,
            "flushing accounts…",
        );
        for batch in self
            .modified_account_ids
            .drain()
            .chunks(BATCH_SIZE)
            .into_iter()
        {
            info!("flushing the batch…");
            let accounts: crate::Result<Vec<(i32, Vec<u8>)>> = batch
                .map(|account_id| {
                    let factors = self
                        .account_cache
                        .peek(&account_id)
                        .expect("the account must be present in the cache");
                    Ok((account_id, rmp_serde::to_vec(factors)?))
                })
                .collect();
            self.redis
                .hset_multiple(ACCOUNT_FACTORS_KEY, &accounts?)
                .await
                .context("failed to flush the accounts factors")?;
        }
        self.account_cache.resize(self.opts.account_cache_size);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn force_flush_vehicles(&mut self) -> crate::Result {
        let vehicles: crate::Result<Vec<(u16, Vec<u8>)>> = self
            .vehicle_cache
            .iter()
            .map(|(tank_id, factors)| Ok((*tank_id, rmp_serde::to_vec(factors)?)))
            .collect();
        self.redis
            .hset_multiple(VEHICLE_FACTORS_KEY, &vehicles?)
            .await
            .context("failed to flush the vehicles factors")?;
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

async fn get_regularization(redis: &mut MultiplexedConnection) -> crate::Result<f64> {
    Ok(redis
        .get::<_, Option<f64>>(REGULARIZATION_KEY)
        .await
        .context("failed to retrieve the model's regularization")?
        .unwrap_or(0.0))
}

async fn set_regularization(
    redis: &mut MultiplexedConnection,
    regularization: f64,
) -> crate::Result {
    redis
        .set(REGULARIZATION_KEY, regularization)
        .await
        .context("failed to update the model's regularization")
}
