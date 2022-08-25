use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::AddAssign;
use std::sync::Arc;

use ahash::AHashMap;
use bpci::{Interval, NSuccessesSample, WilsonScore};
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use nalgebra_sparse::csc::CscCol;
use nalgebra_sparse::{CooMatrix, CscMatrix};
use tokio::spawn;
use tokio::time::sleep;

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
        pointer = {
            let since = now() - Duration::from_std(opts.train_period)?;

            info!(n_train_items = train_set.len(), "evicting outdated items…");
            train_set.retain(|item| item.last_battle_time >= since);

            let mut stream = database::TrainItem::get_stream(&db, since, &pointer).await?;

            info!("reading new items…");
            while let Some(item) = stream.try_next().await? {
                train_set.push(item);
            }

            *train_set
                .iter()
                .map(|item| &item.object_id)
                .max()
                .ok_or_else(|| anyhow!("failed to find the maximum train item's object ID"))?
        };

        let similarities = {
            let (by_vehicle, by_account_tank) = aggregate_train_set(&train_set);

            info!(n_vehicles = by_vehicle.len(), "calculating per vehicle victory ratios…");
            let by_vehicle = calculate_victory_ratios(by_vehicle, z_level);

            let (ratings, tank_ids) = {
                info!(n_tanks = by_account_tank.len(), "calculating per tank victory ratios…");
                let by_account_tank = calculate_victory_ratios(by_account_tank, z_level);
                let ratings = build_matrix(&by_vehicle, by_account_tank);
                let tank_ids = by_vehicle.keys().copied().collect_vec();
                (ratings, tank_ids)
            };

            calculate_similarities(ratings, &tank_ids, opts.buffering).await?
        };

        update_database(&db, similarities).await?;

        info!(?opts.train_interval, %pointer, "sleeping…");
        sleep(opts.train_interval).await;
    }
}

#[derive(Copy, Clone)]
struct Sample {
    n_battles: u32,
    n_wins: u32,
}

impl From<&database::TrainItem> for Sample {
    fn from(item: &database::TrainItem) -> Self {
        Self {
            n_battles: item.n_battles,
            n_wins: item.n_wins,
        }
    }
}

impl AddAssign<&Self> for Sample {
    fn add_assign(&mut self, rhs: &Self) {
        self.n_battles += rhs.n_battles;
        self.n_wins += rhs.n_wins;
    }
}

impl Sample {
    fn victory_ratio(self, z_level: f64) -> Result<f64> {
        Ok(NSuccessesSample::new(self.n_battles, self.n_wins)?
            .wilson_score_with_cc(z_level)
            .mean())
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
    info!(n_battles, elapsed = ?start_instant.elapsed(), "account⨯tank ready");

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
async fn calculate_similarities(
    ratings: CscMatrix<f64>,
    tank_ids: &[wargaming::TankId],
    buffering: usize,
) -> Result<AHashMap<wargaming::TankId, Vec<(wargaming::TankId, f64)>>> {
    info!(nnz = ratings.nnz(), n_vehicles = tank_ids.len(), "calculating similarities…");
    let start_instant = Instant::now();

    let vehicle_magnitudes: Vec<_> = tank_ids
        .iter()
        .copied()
        .map(|tank_id| (tank_id, magnitude(&ratings.col(tank_id as usize))))
        .collect();

    let iter_vehicle_pairs = vehicle_magnitudes
        .iter()
        .flat_map(|(tank_id_1, magnitude_1)| {
            vehicle_magnitudes.iter().map(|(tank_id_2, magnitude_2)| {
                (*tank_id_1, *magnitude_1, *tank_id_2, *magnitude_2)
            })
        })
        .filter(|(tank_id_1, _, tank_id_2, _)| tank_id_2 < tank_id_1);

    let train_set = Arc::new(ratings);
    let mut stream = stream::iter(iter_vehicle_pairs)
        .map(|(tank_id_1, magnitude_1, tank_id_2, magnitude_2)| {
            let train_set = Arc::clone(&train_set);
            spawn(async move {
                let column_1 = train_set.col(tank_id_1 as usize);
                let column_2 = train_set.col(tank_id_2 as usize);
                let similarity = dot_product(&column_1, &column_2) / magnitude_1 / magnitude_2;
                (tank_id_1, tank_id_2, similarity)
            })
        })
        .buffer_unordered(buffering);

    let mut similarities = AHashMap::default();
    while let Some((tank_id_1, tank_id_2, similarity)) = stream.try_next().await? {
        if !similarity.is_finite() {
            continue;
        }
        match similarities.entry(tank_id_1) {
            Entry::Vacant(entry) => {
                entry.insert(vec![(tank_id_2, similarity)]);
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().push((tank_id_2, similarity));
            }
        }
        match similarities.entry(tank_id_2) {
            Entry::Vacant(entry) => {
                entry.insert(vec![(tank_id_1, similarity)]);
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().push((tank_id_1, similarity));
            }
        }
    }

    info!(elapsed = ?start_instant.elapsed());
    Ok(similarities)
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
async fn update_database(
    db: &Database,
    similarities: impl IntoIterator<
        Item = (wargaming::TankId, impl IntoIterator<Item = (wargaming::TankId, f64)>),
    >,
) -> Result {
    info!("updating the database…");
    let start_instant = Instant::now();
    for (source_id, entries) in similarities {
        let similar_vehicles = entries
            .into_iter()
            .filter(|(_, similarity)| *similarity > 0.0)
            .map(|(target_id, similarity)| database::SimilarVehicle {
                tank_id: target_id,
                similarity,
            })
            .collect();
        let model = database::VehicleModel {
            tank_id: source_id,
            similar: similar_vehicles,
        };
        model.upsert(db).await?;
    }
    info!(elapsed = ?start_instant.elapsed(), "updated");
    Ok(())
}
