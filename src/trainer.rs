use std::collections::hash_map::Entry;

use bpci::{Interval, NSuccessesSample, WilsonScore};
use futures::{Stream, TryStreamExt};
use itertools::Itertools;
use nalgebra_sparse::{CooMatrix, CscMatrix};

use crate::database::mongodb::traits::TypedDocument;
use crate::math::statistics::ConfidenceLevel;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, tankopedia, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    let db = database::mongodb::open(&opts.connections.mongodb_uri).await?;
    let deadline = now() - Duration::from_std(opts.train_period)?;

    info!(?deadline, "querying the vehicle stats…");
    let vehicle_stats = database::VehicleStats::retrieve_all(&db, opts.realm, deadline).await?;
    info!(n_vehicles = vehicle_stats.len(), "vehicle stats ready");

    let estimated_document_count = database::TrainItem::collection(&db)
        .estimated_document_count(None)
        .await?;

    info!(estimated_document_count, "querying the train set…");
    let train_set = database::TrainItem::retrieve_all(&db, opts.realm, deadline).await?;

    let train_set = read_matrix(&vehicle_stats, train_set, opts.confidence_level).await?;
    info!(nonzero_proportion = (train_set.nnz() as f64 / estimated_document_count as f64));

    let similarities = calculate_similarities(&train_set, &vehicle_stats);

    for (tank_id_1, tank_id_2, similarity) in similarities
        .into_iter()
        .flat_map(|(tank_id_1, entries)| {
            entries
                .into_iter()
                .filter(move |(tank_id_2, _)| *tank_id_2 < tank_id_1)
                .map(move |(tank_id_2, similarity)| (tank_id_1, tank_id_2, similarity))
        })
        .sorted_by(|(_, _, similarity_1), (_, _, similarity_2)| {
            similarity_2.total_cmp(similarity_1)
        })
        .take(100)
    {
        let vehicle_1 = tankopedia::get_vehicle(tank_id_1);
        let vehicle_2 = tankopedia::get_vehicle(tank_id_2);
        info!(name_1 = ?vehicle_1.name, name_2 = ?vehicle_2.name, similarity);
    }

    Ok(())
}

#[instrument(level = "info", skip_all, fields(confidence_level = ?confidence_level))]
async fn read_matrix(
    vehicle_stats: &AHashMap<wargaming::TankId, database::VehicleStats>,
    mut train_set: impl Stream<Item = Result<database::TrainItem>> + Unpin,
    confidence_level: ConfidenceLevel,
) -> Result<CscMatrix<f64>> {
    let z_level = confidence_level.z_value();

    let mut matrix = CooMatrix::new(u32::MAX as usize, u16::MAX as usize);
    while let Some(item) = train_set.try_next().await? {
        if item.n_battles == 0 {
            continue;
        }
        if let Some(vehicle_stats) = vehicle_stats.get(&item.tank_id) {
            let interval_mean = NSuccessesSample::new(item.n_battles, item.n_wins)?
                .wilson_score_with_cc(z_level)
                .mean();
            matrix.push(
                item.account_id as usize,
                item.tank_id as usize,
                interval_mean - vehicle_stats.victory_ratio,
            );
        }
    }

    info!(matrix.nnz = matrix.nnz(), "almost done…");
    Ok(CscMatrix::from(&matrix))
}

#[instrument(level = "info", skip_all)]
fn calculate_similarities(
    train_set: &CscMatrix<f64>,
    vehicle_stats: &AHashMap<wargaming::TankId, database::VehicleStats>,
) -> AHashMap<wargaming::TankId, Vec<(wargaming::TankId, f64)>> {
    info!(train_set.nnz = train_set.nnz());

    let mut similarities = AHashMap::default();
    for tank_id_1 in vehicle_stats.keys().copied() {
        for tank_id_2 in vehicle_stats.keys().copied() {
            if tank_id_2 >= tank_id_1 {
                continue;
            }
            let similarity = similarity(train_set, tank_id_1, tank_id_2);
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
    }
    similarities
}

fn similarity(
    train_set: &CscMatrix<f64>,
    tank_id_1: wargaming::TankId,
    tank_id_2: wargaming::TankId,
) -> f64 {
    let col_1 = train_set.col(tank_id_1 as usize);
    let col_2 = train_set.col(tank_id_2 as usize);

    let users_1 = col_1
        .row_indices()
        .iter()
        .copied()
        .zip(col_1.values().iter().copied());
    let users_2 = col_2
        .row_indices()
        .iter()
        .copied()
        .zip(col_2.values().iter().copied());
    let numerator: f64 = users_1
        .merge_join_by(users_2, |(i, _), (j, _)| i.cmp(j))
        .filter_map(|item| item.both().map(|((_, x), (_, y))| (x, y)))
        .map(|(x, y)| x * y)
        .sum();
    let denominator_1: f64 = col_1.values().iter().map(|x| x * x).sum();
    let denominator_2: f64 = col_2.values().iter().map(|y| y * y).sum();
    numerator / denominator_1.sqrt() / denominator_2.sqrt()
}
