mod api;
mod client;
mod model;
mod regression;
mod requests;
mod responses;
mod sample;

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
        let (returned_train_set, tank_samples) = spawn(aggregate_train_set(train_set)).await?;
        train_set = returned_train_set;
        spawn(update_model(tank_samples, n_min_points_per_regression, model.clone())).await??;

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

type IndexedByTank<V> =
    AHashMap<wargaming::TankId, AHashMap<(wargaming::Realm, wargaming::AccountId), V>>;

#[instrument(level = "info", skip_all)]
async fn aggregate_train_set(
    train_set: Vec<database::TrainItem>,
) -> (Vec<database::TrainItem>, IndexedByTank<Sample>) {
    let start_instant = Instant::now();
    info!(n_train_items = train_set.len(), "aggregating…");

    let mut n_battles: u32 = 0;

    let mut samples: IndexedByTank<Sample> = AHashMap::default();
    for item in &train_set {
        n_battles += item.n_battles as u32;
        *samples
            .entry(item.tank_id)
            .or_default()
            .entry((item.realm, item.account_id))
            .or_default() += &Sample::from(item);
    }

    info!(n_battles, elapsed = ?start_instant.elapsed(), "completed");
    (train_set, samples)
}

#[instrument(level = "info", skip_all)]
async fn update_model(
    samples: IndexedByTank<Sample>,
    n_min_points_per_regression: usize,
    model: Arc<RwLock<Model>>,
) -> Result {
    info!(n_min_points_per_regression, "updating the model…");
    let start_instant = Instant::now();
    let mut n_vehicle_pairs = 0;
    let mut n_failed_regressions = 0;
    let mut n_points = 0;

    for (n_vehicle, (source_vehicle_id, source_accounts)) in samples.iter().enumerate() {
        if n_vehicle % 25 == 0 {
            info!(n_vehicle, of = samples.len(), n_vehicle_pairs, n_points, n_failed_regressions);
        }
        for (target_vehicle_id, target_accounts) in &samples {
            if source_vehicle_id == target_vehicle_id {
                continue;
            }
            // Contains matrices for this pair of vehicles per realm.
            let mut matrices = AHashMap::default();
            for ((realm, source_account_id), source_sample) in source_accounts {
                let target_sample = match target_accounts.get(&(*realm, *source_account_id)) {
                    Some(sample) => sample,
                    _ => {
                        continue;
                    }
                };
                let (x, y, w) = matrices.remove(realm).unwrap_or_else(|| {
                    (DVector::<f64>::zeros(0), DVector::<f64>::zeros(0), DVector::<f64>::zeros(0))
                });
                let i = x.nrows();
                let x = x.insert_row(i, source_sample.posterior_mean());
                let y = y.insert_row(i, target_sample.posterior_mean());
                let w = w.insert_row(
                    i,
                    source_sample.n_posterior_battles_f64()
                        + target_sample.n_posterior_battles_f64(),
                );
                matrices.insert(*realm, (x, y, w));
            }
            for (realm, (mut x, mut y, w)) in matrices {
                let result = if x.nrows() < n_min_points_per_regression {
                    None
                } else {
                    x.apply(|x| {
                        *x = logit(*x);
                    });
                    y.apply(|y| {
                        *y = logit(*y);
                    });
                    match make_regression(&x, &y, &w) {
                        Some((bias, k)) => Some((k, bias, x, y, w)),
                        _ => {
                            n_failed_regressions += 1;
                            None
                        }
                    }
                };
                let mut model = model.write().await;
                let target_regressions = model
                    .regressions
                    .entry(realm)
                    .or_default()
                    .entry(*target_vehicle_id)
                    .or_default();
                if let Some((k, bias, x, y, w)) = result {
                    n_points += x.nrows();
                    n_vehicle_pairs += 1;
                    target_regressions.insert(*source_vehicle_id, Regression { k, bias, x, y, w });
                } else {
                    target_regressions.remove(source_vehicle_id);
                }
            }
        }
        yield_now().await;
    }

    info!(n_vehicle_pairs, n_points, n_failed_regressions, elapsed = ?start_instant.elapsed(), "completed");
    Ok(())
}

/// See also: <https://arxiv.org/pdf/1311.1835.pdf>.
fn make_regression(x: &DVector<f64>, y: &DVector<f64>, w: &DVector<f64>) -> Option<(f64, f64)> {
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

    (bias.is_finite() && k.is_finite()).then_some((bias, k))
}
