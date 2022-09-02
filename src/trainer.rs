mod api;
pub mod client;
pub mod model;
pub mod requests;
pub mod responses;
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

use self::sample::*;
use crate::math::logit;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::trainer::model::{Model, Regression};
use crate::{database, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let db = database::mongodb::open(&opts.mongodb_uri).await?;
    let model = Arc::new(RwLock::new(Model::default()));

    let serve_api = api::run(&opts.host, opts.port, model.clone());
    let loop_trainer =
        run_trainer(db, Duration::from_std(opts.train_period)?, opts.train_interval, model);

    try_join(serve_api, loop_trainer).await?;
    Ok(())
}

async fn run_trainer(
    db: Database,
    train_period: Duration,
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
        spawn(update_model(tank_samples, model.clone())).await??;

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
async fn update_model(samples: IndexedByTank<Sample>, model: Arc<RwLock<Model>>) -> Result {
    info!("updating the model…");
    let start_instant = Instant::now();
    let mut n_successful = 0;
    let mut n_non_invertible = 0;

    for (n_vehicle, (source_vehicle_id, source_accounts)) in samples.iter().enumerate() {
        if n_vehicle % 25 == 0 {
            info!(n_vehicle, of = samples.len(), n_successful, n_non_invertible);
        }
        for (target_vehicle_id, target_accounts) in &samples {
            if source_vehicle_id == target_vehicle_id {
                continue;
            }
            let mut matrices = AHashMap::default();
            for ((realm, source_account_id), source_sample) in source_accounts {
                let target_sample = match target_accounts.get(&(*realm, *source_account_id)) {
                    Some(sample) => sample,
                    _ => {
                        continue;
                    }
                };
                let (x, y) = matrices
                    .remove(realm)
                    .unwrap_or_else(|| (DVector::<f64>::zeros(0), DVector::<f64>::zeros(0)));
                let i = x.nrows();
                let x = x.insert_row(i, source_sample.mean());
                let y = y.insert_row(i, target_sample.mean());
                matrices.insert(*realm, (x, y));
            }
            for (realm, (mut x, mut y)) in matrices {
                debug_assert_ne!(x.nrows(), 0);
                x.apply(|x| {
                    *x = logit(*x);
                });
                let x = x.insert_column(1, 1.0);
                y.apply(|y| {
                    *y = logit(*y);
                });
                let theta = match (x.tr_mul(&x)).try_inverse() {
                    Some(inverted) => inverted * x.transpose() * y,
                    _ => {
                        n_non_invertible += 1;
                        continue;
                    }
                };
                n_successful += 1;
                model
                    .write()
                    .await
                    .regressions
                    .entry(realm)
                    .or_default()
                    .entry(*target_vehicle_id)
                    .or_default()
                    .insert(
                        *source_vehicle_id,
                        Regression {
                            k: theta[0],
                            bias: theta[1],
                            n_rows: x.nrows(),
                        },
                    );
            }
        }
        yield_now().await;
    }

    info!(elapsed = ?start_instant.elapsed(), "completed");
    Ok(())
}
