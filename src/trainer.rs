mod api;
pub mod client;
pub mod requests;
pub mod responses;
mod sample;

use std::sync::Arc;

use futures::future::try_join;
use futures::{Stream, TryStreamExt};
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use tokio::sync::RwLock;
use tokio::time::sleep;

use self::sample::*;
use crate::math::statistics::ConfidenceLevel;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "trainer"));

    let db = database::mongodb::open(&opts.mongodb_uri).await?;

    let vehicle_victory_ratios = Arc::new(RwLock::new(AHashMap::default()));
    let vehicle_similarities = Arc::new(RwLock::new(AHashMap::default()));

    let serve_api = api::run(
        &opts.host,
        opts.port,
        vehicle_victory_ratios.clone(),
        vehicle_similarities.clone(),
    );
    let loop_trainer = run_trainer(
        db,
        opts.confidence_level,
        opts.train_period,
        vehicle_victory_ratios,
        vehicle_similarities,
    );

    try_join(serve_api, loop_trainer).await?;
    Ok(())
}

async fn run_trainer(
    db: Database,
    confidence_level: ConfidenceLevel,
    train_period: time::Duration,
    vehicle_victory_ratios: Arc<RwLock<AHashMap<wargaming::TankId, f64>>>,
    vehicle_similarities: Arc<RwLock<AHashMap<(wargaming::TankId, wargaming::TankId), f64>>>,
) -> Result {
    let z_level = confidence_level.z_value();

    let mut pointer = ObjectId::from_bytes([0; 12]);
    let mut train_set: Vec<database::TrainItem> = Vec::new();
    loop {
        let since = now() - Duration::from_std(train_period)?;

        info!(n_train_items = train_set.len(), "evicting outdated items…");
        train_set.retain(|item| item.last_battle_time >= since);

        let stream = database::TrainItem::get_stream(&db, since, &pointer).await?;
        read_stream(stream, &mut pointer, &mut train_set).await?;
        let (by_vehicle, by_tank_account) = aggregate_train_set(&train_set);
        update_vehicle_victory_ratios(by_vehicle, z_level, &vehicle_victory_ratios).await?;
        let ratings =
            calculate_normalized_ratings(by_tank_account, z_level, &vehicle_victory_ratios).await;
        update_vehicle_similarities(ratings, &vehicle_similarities).await;

        info!(?train_period, %pointer, "sleeping…");
        sleep(train_period).await;
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
            info!(n_read, elapsed = ?start_instant.elapsed());
        }
    }
    info!(elapsed = ?start_instant.elapsed(), n_items = into.len(), "finished");
    Ok(())
}

type IndexedByVehicle<V> = AHashMap<wargaming::TankId, V>;
type IndexedByTank<V> = AHashMap<wargaming::TankId, AHashMap<wargaming::AccountId, V>>;

#[instrument(level = "info", skip_all)]
fn aggregate_train_set(
    train_set: &[database::TrainItem],
) -> (IndexedByVehicle<Sample>, IndexedByTank<Sample>) {
    let start_instant = Instant::now();
    info!(n_train_items = train_set.len(), "aggregating…");

    let mut n_battles = 0;

    let mut by_tank_account: IndexedByTank<Sample> = AHashMap::default();
    for item in train_set {
        n_battles += item.n_battles;
        *by_tank_account
            .entry(item.tank_id)
            .or_default()
            .entry(item.account_id)
            .or_default() += &Sample::from(item);
    }

    let mut by_vehicle = AHashMap::default();
    for (tank_id, by_account) in &by_tank_account {
        for sample in by_account.values() {
            *by_vehicle.entry(*tank_id).or_default() += sample;
        }
    }

    info!(n_battles, elapsed = ?start_instant.elapsed(), "completed");
    (by_vehicle, by_tank_account)
}

#[instrument(level = "info", skip_all)]
async fn update_vehicle_victory_ratios(
    by_vehicle: IndexedByVehicle<Sample>,
    z_level: f64,
    vehicle_victory_ratios: &RwLock<AHashMap<wargaming::TankId, f64>>,
) -> Result {
    info!(n_vehicles = by_vehicle.len(), "updating per vehicle victory ratios…");
    for (tank_id, sample) in by_vehicle {
        vehicle_victory_ratios
            .write()
            .await
            .insert(tank_id, sample.victory_ratio(z_level)?);
    }
    Ok(())
}

#[instrument(level = "info", skip_all)]
async fn calculate_normalized_ratings(
    by_tank_account: IndexedByTank<Sample>,
    z_level: f64,
    vehicle_victory_ratios: &RwLock<AHashMap<wargaming::TankId, f64>>,
) -> IndexedByTank<f64> {
    let start_instant = Instant::now();
    info!("calculating normalized ratings…");

    let mut ratings: AHashMap<wargaming::TankId, AHashMap<wargaming::AccountId, f64>> =
        AHashMap::default();
    for (tank_id, by_account) in by_tank_account {
        if let Some(vehicle_victory_ratio) = vehicle_victory_ratios.read().await.get(&tank_id) {
            let vehicle_victory_ratio = *vehicle_victory_ratio;
            let account_ratings = ratings.entry(tank_id).or_default();
            for (account_id, sample) in by_account {
                match sample.victory_ratio(z_level) {
                    Ok(victory_ratio) => {
                        account_ratings.insert(account_id, victory_ratio - vehicle_victory_ratio);
                    }
                    Err(error) => {
                        warn!("{:#}", error);
                    }
                }
            }
        };
    }

    info!(elapsed = ?start_instant.elapsed(), "completed");
    ratings
}

#[instrument(level = "info", skip_all)]
async fn update_vehicle_similarities(
    ratings: IndexedByTank<f64>,
    vehicle_similarities: &RwLock<AHashMap<(wargaming::TankId, wargaming::TankId), f64>>,
) {
    let start_instant = Instant::now();
    info!("calculating & updating vehicle similarities…");
    for (tank_id_1, by_account_1) in &ratings {
        vehicle_similarities
            .write()
            .await
            .insert((*tank_id_1, *tank_id_1), 1.0);
        let magnitude_1 = magnitude(by_account_1.values());
        for (tank_id_2, by_account_2) in &ratings {
            if tank_id_1 >= tank_id_2 {
                continue;
            }
            let magnitude_2 = magnitude(by_account_2.values());
            let dot_product = by_account_1
                .iter()
                .filter_map(|(account_id, rating_1)| {
                    by_account_2
                        .get(account_id)
                        .map(|rating_2| rating_1 * rating_2)
                })
                .sum::<f64>();
            let similarity = dot_product / magnitude_1 / magnitude_2;
            if similarity.is_finite() {
                let mut vehicle_similarities = vehicle_similarities.write().await;
                if similarity > 0.0 {
                    vehicle_similarities.insert((*tank_id_1, *tank_id_2), similarity);
                    vehicle_similarities.insert((*tank_id_2, *tank_id_1), similarity);
                } else {
                    vehicle_similarities.remove(&(*tank_id_1, *tank_id_2));
                    vehicle_similarities.remove(&(*tank_id_2, *tank_id_1));
                }
            }
        }
    }
    info!(elapsed = ?start_instant.elapsed(), "completed");
}

fn magnitude<'a>(vector: impl IntoIterator<Item = &'a f64>) -> f64 {
    vector.into_iter().map(|x| x * x).sum::<f64>().sqrt()
}
