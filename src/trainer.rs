mod attributes;
mod sample;

use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use ahash::AHashMap;
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use nalgebra_sparse::csc::CscCol;
use nalgebra_sparse::{CooMatrix, CscMatrix};
use tokio::spawn;
use tokio::time::sleep;

use self::attributes::*;
use self::sample::*;
use crate::database::mongodb::traits::Upsert;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    let db = database::mongodb::open(&opts.mongodb_uri).await?;
    let z_level = opts.confidence_level.z_value();

    let mut pointer = ObjectId::from_bytes([0; 12]);
    let mut train_set: Vec<database::TrainItem> = Vec::new();
    loop {
        {
            let since = now() - Duration::from_std(opts.train_period)?;

            info!(n_train_items = train_set.len(), "evicting outdated items…");
            train_set.retain(|item| item.last_battle_time >= since);

            let mut stream = database::TrainItem::get_stream(&db, since, &pointer).await?;
            info!("reading new items…");
            while let Some(item) = stream.try_next().await? {
                if pointer < item.object_id {
                    pointer = item.object_id;
                }
                train_set.push(item);
            }
        }

        let models = {
            let (by_vehicle, by_account_tank) = aggregate_train_set(&train_set);

            info!(n_vehicles = by_vehicle.len(), "calculating per vehicle victory ratios…");
            let by_vehicle = calculate_victory_ratios(by_vehicle, z_level);

            let ratings = {
                info!(n_tanks = by_account_tank.len(), "calculating per tank victory ratios…");
                let by_account_tank = calculate_victory_ratios(by_account_tank, z_level);
                build_matrix(&by_vehicle, by_account_tank)
            };

            build_models(ratings, by_vehicle, opts.buffering).await?
        };

        update_database(&db, models).await?;

        info!(?opts.train_interval, %pointer, "sleeping…");
        sleep(opts.train_interval).await;
    }
}

#[instrument(level = "info", skip_all)]
fn aggregate_train_set(
    train_set: &[database::TrainItem],
) -> (
    AHashMap<wargaming::TankId, Sample>,
    AHashMap<(wargaming::TankId, wargaming::AccountId), Sample>,
) {
    info!(n_items = train_set.len(), "aggregating…");

    let mut by_account_tank = AHashMap::default();
    let mut n_battles = 0;
    let start_instant = Instant::now();

    for item in train_set {
        n_battles += item.n_battles;
        let sample = Sample::from(item);

        match by_account_tank.entry((item.tank_id, item.account_id)) {
            Entry::Vacant(entry) => {
                entry.insert(sample);
            }
            Entry::Occupied(mut entry) => {
                *entry.get_mut() += &sample;
            }
        }
    }

    let mut by_vehicle = AHashMap::default();
    for ((tank_id, _), sample) in &by_account_tank {
        match by_vehicle.entry(*tank_id) {
            Entry::Vacant(entry) => {
                entry.insert(*sample);
            }
            Entry::Occupied(mut entry) => {
                *entry.get_mut() += sample;
            }
        }
    }

    info!(n_battles, elapsed = ?start_instant.elapsed(), "finished");
    (by_vehicle, by_account_tank)
}

#[instrument(level = "info", skip_all)]
fn calculate_victory_ratios<K: Eq + Hash + Debug>(
    mapping: AHashMap<K, Sample>,
    z_level: f64,
) -> AHashMap<K, f64> {
    mapping
        .into_iter()
        .filter_map(|(key, sample)| match sample.victory_ratio(z_level) {
            Ok(victory_ratio) => Some((key, victory_ratio)),
            Err(error) => {
                warn!(?key, "{:?}", error);
                None
            }
        })
        .collect()
}

#[instrument(level = "info", skip_all)]
fn build_matrix(
    by_vehicle: &AHashMap<wargaming::TankId, f64>,
    by_account_tank: AHashMap<(wargaming::TankId, wargaming::AccountId), f64>,
) -> CscMatrix<f64> {
    info!(n_account_tanks = by_account_tank.len(), "building matrix…");
    let start_instant = Instant::now();
    let mut matrix = CooMatrix::new(u32::MAX as usize, u16::MAX as usize);

    for ((tank_id, account_id), victory_ratio) in by_account_tank {
        if let Some(vehicle_victory_ratio) = by_vehicle.get(&tank_id) {
            matrix.push(
                account_id as usize,
                tank_id as usize,
                victory_ratio - vehicle_victory_ratio,
            );
        }
    }

    info!(matrix.nnz = matrix.nnz(), elapsed = ?start_instant.elapsed(), "converting…");
    CscMatrix::from(&matrix)
}

#[instrument(level = "info", skip_all)]
async fn build_models(
    ratings: CscMatrix<f64>,
    by_vehicle: AHashMap<wargaming::TankId, f64>,
    buffering: usize,
) -> Result<Vec<database::VehicleModel>> {
    info!(nnz = ratings.nnz(), n_vehicles = by_vehicle.len(), "building vehicle models…");
    let start_instant = Instant::now();

    let vehicle_attributes: Vec<_> = by_vehicle
        .into_iter()
        .map(|(tank_id, victory_ratio)| VehicleAttributes {
            tank_id,
            victory_ratio,
            magnitude: magnitude(&ratings.col(tank_id as usize)),
        })
        .collect();

    let vehicle_pairs = vehicle_attributes
        .iter()
        .flat_map(|attrs_1| {
            vehicle_attributes
                .iter()
                .map(|attrs_2| (*attrs_1, *attrs_2))
        })
        .filter(|(attrs_1, attrs_2)| attrs_2.tank_id < attrs_1.tank_id);

    let train_set = Arc::new(ratings);
    let mut stream = stream::iter(vehicle_pairs)
        .map(|(attrs_1, attrs_2)| {
            let train_set = Arc::clone(&train_set);
            spawn(async move {
                let column_1 = train_set.col(attrs_1.tank_id as usize);
                let column_2 = train_set.col(attrs_2.tank_id as usize);
                let similarity =
                    dot_product(&column_1, &column_2) / attrs_1.magnitude / attrs_2.magnitude;
                (attrs_1, attrs_2, similarity)
            })
        })
        .buffer_unordered(buffering);

    let mut models = AHashMap::default();
    while let Some((attrs_1, attrs_2, similarity)) = stream.try_next().await? {
        if !similarity.is_finite() || similarity <= 0.0 {
            continue;
        }
        for (attrs_1, attrs_2) in [(attrs_1, attrs_2), (attrs_2, attrs_1)] {
            let vehicle_2 = database::SimilarVehicle {
                tank_id: attrs_2.tank_id,
                similarity,
            };
            match models.entry(attrs_1.tank_id) {
                Entry::Vacant(entry) => {
                    entry.insert(database::VehicleModel {
                        tank_id: attrs_1.tank_id,
                        victory_ratio: attrs_1.victory_ratio,
                        similar: vec![vehicle_2],
                    });
                }
                Entry::Occupied(mut entry) => {
                    entry.get_mut().similar.push(vehicle_2);
                }
            }
        }
    }

    info!(elapsed = ?start_instant.elapsed());
    Ok(models.into_iter().map(|(_, model)| model).collect())
}

fn dot_product(column_1: &CscCol<f64>, column_2: &CscCol<f64>) -> f64 {
    let values_1 = column_1.row_indices().iter().zip(column_1.values().iter());
    let values_2 = column_2.row_indices().iter().zip(column_2.values().iter());
    values_1
        .merge_join_by(values_2, |(i, _), (j, _)| i.cmp(j))
        .filter_map(|item| item.both().map(|((_, x), (_, y))| (x, y)))
        .map(|(x, y)| x * y)
        .sum::<f64>()
}

fn magnitude(column: &CscCol<f64>) -> f64 {
    column.values().iter().map(|x| x * x).sum::<f64>().sqrt()
}

#[instrument(level = "info", skip_all)]
async fn update_database(db: &Database, models: Vec<database::VehicleModel>) -> Result {
    info!("updating the database…");
    let start_instant = Instant::now();
    for model in models {
        model.upsert(db).await?;
    }
    info!(elapsed = ?start_instant.elapsed(), "updated");
    Ok(())
}
