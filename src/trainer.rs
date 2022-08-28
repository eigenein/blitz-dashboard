mod api;
pub mod client;
pub mod model;
pub mod requests;
pub mod responses;
mod sample;
mod train_item;

use std::sync::Arc;

use futures::future::try_join;
use futures::{Stream, TryStreamExt};
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use tokio::sync::RwLock;
use tokio::time::sleep;

use self::sample::*;
use crate::math::logit;
use crate::math::statistics::ConfidenceLevel;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::trainer::model::Model;
use crate::trainer::train_item::CompressedTrainItem;
use crate::{database, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let db = database::mongodb::open(&opts.mongodb_uri).await?;
    let model = Arc::new(RwLock::new(Model::default()));

    let serve_api = api::run(&opts.host, opts.port, model.clone());
    let loop_trainer = run_trainer(db, opts.confidence_level, opts.train_period, model);

    try_join(serve_api, loop_trainer).await?;
    Ok(())
}

async fn run_trainer(
    db: Database,
    confidence_level: ConfidenceLevel,
    train_period: time::Duration,
    model: Arc<RwLock<Model>>,
) -> Result {
    let z_level = confidence_level.z_value();

    let mut pointer = ObjectId::from_bytes([0; 12]);
    let mut train_set: Vec<CompressedTrainItem> = Vec::new();
    loop {
        let since = now() - Duration::from_std(train_period)?;

        info!(n_train_items = train_set.len(), "evicting outdated items…");
        train_set.retain(|item| item.last_battle_time >= since.timestamp());

        let stream = database::TrainItem::get_stream(&db, since, &pointer).await?;
        read_stream(stream, &mut pointer, &mut train_set).await?;
        let tank_samples = aggregate_train_set(&train_set);
        let mut ratings = calculate_ratings(&tank_samples, z_level).await;
        calculate_vehicle_mean_ratings(&tank_samples, &model, z_level).await?;
        normalize_ratings(&mut ratings, &model).await?;
        update_vehicle_similarities(ratings, &model).await;

        info!(?train_period, %pointer, "sleeping…");
        sleep(train_period).await;
    }
}

#[instrument(level = "info", skip_all)]
async fn read_stream(
    mut stream: impl Stream<Item = Result<database::TrainItem>> + Unpin,
    pointer: &mut ObjectId,
    into: &mut Vec<CompressedTrainItem>,
) -> Result {
    let start_instant = Instant::now();
    info!("reading new items…");

    let mut n_read = 0;
    while let Some(item) = stream.try_next().await? {
        if *pointer < item.object_id {
            *pointer = item.object_id;
        }
        into.push(item.try_into()?);
        n_read += 1;
        if n_read % 100_000 == 0 {
            info!(n_read, elapsed_secs = ?start_instant.elapsed().as_secs(), per_sec = n_read / start_instant.elapsed().as_secs().max(1));
        }
    }

    info!(elapsed = ?start_instant.elapsed(), n_items = into.len(), "finished");
    Ok(())
}

type IndexedByTank<V> = AHashMap<wargaming::TankId, AHashMap<wargaming::AccountId, V>>;

#[instrument(level = "info", skip_all)]
fn aggregate_train_set(train_set: &[CompressedTrainItem]) -> IndexedByTank<Sample> {
    let start_instant = Instant::now();
    info!(n_train_items = train_set.len(), "aggregating…");

    let mut n_battles: u32 = 0;

    let mut tank_samples: IndexedByTank<Sample> = AHashMap::default();
    for item in train_set {
        n_battles += item.n_battles as u32;
        *tank_samples
            .entry(item.tank_id)
            .or_default()
            .entry(item.account_id)
            .or_default() += &Sample::from(item);
    }

    info!(n_battles, elapsed = ?start_instant.elapsed(), "completed");
    tank_samples
}

#[instrument(level = "info", skip_all)]
async fn calculate_ratings(
    tank_samples: &IndexedByTank<Sample>,
    z_level: f64,
) -> IndexedByTank<f64> {
    let start_instant = Instant::now();
    info!("calculating ratings…");

    let mut ratings: AHashMap<wargaming::TankId, AHashMap<wargaming::AccountId, f64>> =
        AHashMap::default();
    for (tank_id, account_samples) in tank_samples {
        let account_ratings = ratings.entry(*tank_id).or_default();
        for (account_id, sample) in account_samples {
            match sample.victory_ratio(z_level) {
                Ok(victory_ratio) => {
                    account_ratings.insert(*account_id, logit(victory_ratio));
                }
                Err(error) => {
                    warn!("{:#}", error);
                }
            }
        }
    }

    info!(elapsed = ?start_instant.elapsed(), "completed");
    ratings
}

#[instrument(level = "info", skip_all)]
async fn calculate_vehicle_mean_ratings(
    ratings: &IndexedByTank<Sample>,
    model: &RwLock<Model>,
    z_level: f64,
) -> Result {
    let start_instant = Instant::now();
    info!("calculating vehicle mean ratings…");

    for (tank_id, account_ratings) in ratings {
        let total_sample = account_ratings.values().sum::<Sample>();
        let mean_rating = total_sample.victory_ratio(z_level)?;
        model
            .write()
            .await
            .vehicles
            .entry(*tank_id)
            .or_default()
            .mean_rating = logit(mean_rating);
    }

    info!(elapsed = ?start_instant.elapsed(), "completed");
    Ok(())
}

#[instrument(level = "info", skip_all)]
async fn normalize_ratings(ratings: &mut IndexedByTank<f64>, model: &RwLock<Model>) -> Result {
    let start_instant = Instant::now();
    info!("normalizing ratings…");

    for (tank_id, account_ratings) in ratings.iter_mut() {
        let vehicle_mean_rating = model
            .read()
            .await
            .vehicles
            .get(tank_id)
            .ok_or_else(|| anyhow!("vehicle #{}'s mean rating is missing", tank_id))?
            .mean_rating;
        for rating in account_ratings.values_mut() {
            *rating -= vehicle_mean_rating;
        }
    }

    info!(elapsed = ?start_instant.elapsed(), "completed");
    Ok(())
}

#[instrument(level = "info", skip_all)]
async fn update_vehicle_similarities(ratings: IndexedByTank<f64>, model: &RwLock<Model>) {
    let start_instant = Instant::now();
    info!("calculating & updating vehicle similarities…");
    for (i, (tank_id_1, account_1_ratings)) in ratings.iter().enumerate() {
        if i % 50 == 0 {
            info!(i, elapsed = ?start_instant.elapsed(), "working…");
        }
        let magnitude_1 = magnitude(account_1_ratings.values());
        for (tank_id_2, account_2_ratings) in &ratings {
            if tank_id_1 >= tank_id_2 {
                continue;
            }
            let magnitude_2 = magnitude(account_2_ratings.values());
            let dot_product = account_1_ratings
                .iter()
                .filter_map(|(account_id, rating_1)| {
                    account_2_ratings
                        .get(account_id)
                        .map(|rating_2| rating_1 * rating_2)
                })
                .sum::<f64>();
            let similarity = dot_product / magnitude_1 / magnitude_2;
            if similarity.is_finite() {
                for (tank_id_1, tank_id_2) in [(tank_id_1, tank_id_2), (tank_id_2, tank_id_1)] {
                    let mut model = model.write().await;
                    let entry_1 = model.vehicles.entry(*tank_id_1).or_default();
                    if similarity > 0.0 {
                        entry_1.similarities.entry(*tank_id_2).or_insert(similarity);
                    } else {
                        entry_1.similarities.remove(tank_id_2);
                    }
                }
            }
        }
    }
    info!(elapsed = ?start_instant.elapsed(), "completed");
}

fn magnitude<'a>(vector: impl IntoIterator<Item = &'a f64>) -> f64 {
    vector.into_iter().map(|x| x * x).sum::<f64>().sqrt()
}
