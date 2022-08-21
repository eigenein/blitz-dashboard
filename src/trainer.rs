use std::collections::hash_map::Entry;

use futures::{Stream, TryStreamExt};
use itertools::Itertools;
use nalgebra_sparse::{CooMatrix, CscMatrix};

use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, tankopedia, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    let db = database::mongodb::open(&opts.connections.mongodb_uri).await?;
    let deadline = now() - Duration::from_std(opts.train_period)?;
    let z_level = opts.confidence_level.z_value();

    info!(?deadline, "querying the vehicle stats…");
    let vehicle_stats =
        database::TrainAggregation::aggregate_by_vehicles(&db, opts.realm, deadline)
            .await
            .context("failed to aggregate vehicle stats")?
            .into_iter()
            .map(|vehicle| Ok((vehicle.tank_id, vehicle.victory_ratio(z_level)?)))
            .collect::<Result<AHashMap<wargaming::TankId, f64>>>()?;
    let tank_ids = vehicle_stats.keys().copied().collect_vec();

    info!(n_vehicles = vehicle_stats.len(), "vehicle stats ready, querying the train set…");

    let train_set =
        database::TrainAggregation::aggregate_by_account_tanks(&db, opts.realm, deadline)
            .await
            .context("failed to aggregate the train set")?;

    let train_set = build_matrix(&vehicle_stats, Box::pin(train_set), z_level).await?;

    let similarities = calculate_similarities(&train_set, &tank_ids);
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
        .take(50)
    {
        let vehicle_1 = tankopedia::get_vehicle(tank_id_1);
        let vehicle_2 = tankopedia::get_vehicle(tank_id_2);
        info!(name_1 = ?vehicle_1.name, name_2 = ?vehicle_2.name, similarity);
    }

    Ok(())
}

#[instrument(level = "info", skip_all)]
async fn build_matrix(
    vehicle_stats: &AHashMap<wargaming::TankId, f64>,
    mut train_set: impl Stream<Item = Result<database::TrainAggregation>> + Unpin,
    z_level: f64,
) -> Result<CscMatrix<f64>> {
    info!(z_level, n_vehicles = vehicle_stats.len());

    let mut matrix = CooMatrix::new(u32::MAX as usize, u16::MAX as usize);
    while let Some(item) = train_set.try_next().await? {
        if let Some(vehicle_victory_ratio) = vehicle_stats.get(&item.tank_id) {
            let value = item.victory_ratio(z_level)? - vehicle_victory_ratio;
            debug_assert!(value.is_finite(), "item = {:?}", item);
            matrix.push(item.account_id as usize, item.tank_id as usize, value);
        }
    }

    info!(matrix.nnz = matrix.nnz(), "COO matrix is ready, converting…");
    Ok(CscMatrix::from(&matrix))
}

#[instrument(level = "info", skip_all)]
fn calculate_similarities(
    train_set: &CscMatrix<f64>,
    tank_ids: &[wargaming::TankId],
) -> AHashMap<wargaming::TankId, Vec<(wargaming::TankId, f64)>> {
    info!(train_set.nnz = train_set.nnz(), n_vehicles = tank_ids.len());

    let mut similarities = AHashMap::default();
    for tank_id_1 in tank_ids.iter().copied() {
        for tank_id_2 in tank_ids.iter().copied() {
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
