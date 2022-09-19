mod api;
mod client;
mod model;
mod regression;
mod requests;
mod responses;
mod sample;

use std::hash::Hash;
use std::sync::Arc;

use futures::future::try_join;
use futures::{Stream, TryStreamExt};
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use nalgebra::DVector;
use tokio::spawn;
use tokio::sync::RwLock;
use tokio::task::yield_now;
use tokio::time::sleep;

pub use self::client::*;
pub use self::model::*;
pub use self::regression::*;
pub use self::requests::*;
pub use self::responses::*;
use self::sample::*;
use crate::math::logit;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let db = database::mongodb::open(&opts.mongodb_uri).await?;
    let model = Arc::new(RwLock::new(Model::default()));

    let serve_api = api::run(&opts.host, opts.port, model.clone());
    let loop_trainer = run_trainer(
        db,
        Duration::from_std(opts.train_period)?,
        opts.n_min_points_per_regression,
        opts.train_interval,
        model,
    );

    try_join(serve_api, loop_trainer).await?;
    Ok(())
}

async fn run_trainer(
    db: Database,
    train_period: Duration,
    n_min_points_per_regression: usize,
    train_interval: time::Duration,
    model: Arc<RwLock<Model>>,
) -> Result {
    let mut pointer = ObjectId::from_bytes([0; 12]);
    let mut train_set: Vec<database::TrainItem> = Vec::new();
    loop {
        let since = now() - train_period;

        info!(n_train_items = train_set.len(), "evicting outdated items…");
        train_set.retain(|item| item.last_battle_time >= since);

        let stream = database::TrainItem::get_stream(&db, since, &pointer).await?;
        read_stream(stream, &mut pointer, &mut train_set).await?;
        let (returned_train_set, samples_by_tank) =
            spawn(aggregate_train_set(train_set, |item| item.tank_id, |item| item.account_id))
                .await?;
        train_set = returned_train_set;
        spawn(update_model(samples_by_tank, n_min_points_per_regression, model.clone())).await??;

        info!(?train_interval, %pointer, "sleeping…");
        sleep(train_interval).await;
    }
}

#[instrument(level = "info", skip_all)]
async fn read_stream(
    mut stream: impl Stream<Item = Result<database::TrainItem>> + Unpin,
    pointer: &mut ObjectId,
    into: &mut Vec<database::TrainItem>,
) -> Result {
    let start_instant = Instant::now();
    info!("reading new items…");

    let mut n_read = 0;
    while let Some(item) = stream.try_next().await? {
        if *pointer < item.object_id {
            *pointer = item.object_id;
        }
        into.push(item);
        n_read += 1;
        if n_read % 1000 == 0 {
            yield_now().await;
        }
        if n_read % 100_000 == 0 {
            info!(n_read, elapsed_secs = ?start_instant.elapsed().as_secs(), per_sec = n_read / start_instant.elapsed().as_secs().max(1));
        }
    }

    info!(elapsed = ?start_instant.elapsed(), n_items = into.len(), "finished");
    Ok(())
}

type IndexedSamples<K1, K2> = AHashMap<wargaming::Realm, AHashMap<K1, AHashMap<K2, Sample>>>;

#[instrument(level = "info", skip_all)]
async fn aggregate_train_set<K1: Eq + Hash, K2: Eq + Hash>(
    train_set: Vec<database::TrainItem>,
    key_1: fn(&database::TrainItem) -> K1,
    key_2: fn(&database::TrainItem) -> K2,
) -> (Vec<database::TrainItem>, IndexedSamples<K1, K2>) {
    let start_instant = Instant::now();
    info!(n_train_items = train_set.len(), "aggregating…");

    let mut n_battles: u32 = 0;
    let mut aggregated: IndexedSamples<K1, K2> = AHashMap::default();
    for item in &train_set {
        n_battles += item.n_battles as u32;
        *aggregated
            .entry(item.realm)
            .or_default()
            .entry(key_1(item))
            .or_default()
            .entry(key_2(item))
            .or_default() += &Sample::from(item);
    }

    info!(n_battles, elapsed = ?start_instant.elapsed(), "completed");
    (train_set, aggregated)
}

#[instrument(level = "info", skip_all)]
async fn update_model(
    samples_by_tank: IndexedSamples<wargaming::TankId, wargaming::AccountId>,
    n_min_points_per_regression: usize,
    model: Arc<RwLock<Model>>,
) -> Result {
    info!(n_min_points_per_regression, "updating the model…");
    let start_instant = Instant::now();
    let mut n_regressions = 0;
    let mut n_failed_regressions = 0;
    let mut n_points = 0;

    for (realm, samples_by_tank) in &samples_by_tank {
        for (n_vehicle, (target_vehicle_id, target_accounts)) in samples_by_tank.iter().enumerate()
        {
            if n_vehicle % 25 == 0 {
                info!(
                    n_vehicle,
                    of = samples_by_tank.len(),
                    n_regressions,
                    n_points,
                    n_failed_regressions
                );
            }

            for (source_vehicle_id, source_accounts) in samples_by_tank {
                if source_vehicle_id == target_vehicle_id {
                    continue;
                }
                let mut matrices = Matrices::default();
                for (source_account_id, source_sample) in source_accounts {
                    if let Some(target_sample) = target_accounts.get(source_account_id) {
                        matrices = update_matrices(*source_sample, *target_sample, matrices);
                    }
                }
                {
                    let result = make_regression(&mut matrices, n_min_points_per_regression);
                    if result == RegressionResult::Undefined {
                        n_failed_regressions += 1;
                    }
                    let mut model = model.write().await;
                    let target_regressions = model
                        .regressions
                        .entry(*realm)
                        .or_default()
                        .entry(*target_vehicle_id)
                        .or_default();
                    let Matrices { x, y, w } = matrices;
                    if let RegressionResult::Ok { bias, k } = result {
                        n_points += x.nrows();
                        n_regressions += 1;
                        target_regressions
                            .insert(*source_vehicle_id, Regression { k, bias, x, y, w });
                    } else {
                        target_regressions.remove(source_vehicle_id);
                    }
                }
            }
        }
        yield_now().await;
    }

    info!(n_regressions, n_points, n_failed_regressions, elapsed = ?start_instant.elapsed(), "completed");
    Ok(())
}

struct Matrices {
    x: DVector<f64>,
    y: DVector<f64>,
    w: DVector<f64>,
}

impl Default for Matrices {
    fn default() -> Self {
        Self {
            x: DVector::zeros(0),
            y: DVector::zeros(0),
            w: DVector::zeros(0),
        }
    }
}

fn update_matrices(source_sample: Sample, target_sample: Sample, matrices: Matrices) -> Matrices {
    let i = matrices.x.nrows();
    let x = matrices.x.insert_row(i, source_sample.posterior_mean());
    let y = matrices.y.insert_row(i, target_sample.posterior_mean());
    let w = matrices.w.insert_row(
        i,
        source_sample.n_posterior_battles_f64() + target_sample.n_posterior_battles_f64(),
    );
    Matrices { x, y, w }
}

#[derive(PartialEq)]
enum RegressionResult {
    Ok { bias: f64, k: f64 },
    NotEnoughPoints,
    Undefined,
}

/// See also: <https://arxiv.org/pdf/1311.1835.pdf>.
#[instrument(level = "debug", skip_all)]
fn make_regression(
    matrices: &mut Matrices,
    n_min_points_per_regression: usize,
) -> RegressionResult {
    let Matrices { x, y, w } = matrices;

    if x.nrows() < n_min_points_per_regression {
        return RegressionResult::NotEnoughPoints;
    }

    x.apply(|x| {
        *x = logit(*x);
    });
    y.apply(|y| {
        *y = logit(*y);
    });

    let total_weight = w.sum();
    let mean_x = x.dot(w) / total_weight;
    let mean_y = y.dot(w) / total_weight;
    let normalized_x = x.add_scalar(-mean_x);
    let normalized_y = y.add_scalar(-mean_y);

    let k = {
        let numerator = normalized_x.component_mul(&normalized_y).dot(w);
        let denominator = normalized_x.component_mul(&normalized_x).dot(w);
        numerator / denominator
    };

    let bias = mean_y - k * mean_x;

    if bias.is_finite() && k.is_finite() {
        RegressionResult::Ok { bias, k }
    } else {
        RegressionResult::Undefined
    }
}
