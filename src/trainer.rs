use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::AddAssign;
use std::sync::Arc;

use ahash::AHashMap;
use bpci::{Interval, NSuccessesSample, WilsonScore};
use futures::future::ready;
use futures::{stream, Stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use nalgebra_sparse::csc::CscCol;
use nalgebra_sparse::{CooMatrix, CscMatrix};
use tokio::spawn;

use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, tankopedia, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    let db = database::mongodb::open(&opts.connections.mongodb_uri).await?;

    let since = now() - Duration::from_std(opts.train_period)?;
    info!(?since, "querying train set…");

    let train_set = database::TrainItem::stream(&db, opts.realm, since).await?;
    let (by_vehicle, by_account_tank) = aggregate_train_set(Box::pin(train_set)).await?;

    let z_level = opts.confidence_level.z_value();

    info!(n_vehicles = by_vehicle.len(), "calculating vehicle victory ratios…");
    let by_vehicle = calculate_victory_ratios(by_vehicle, z_level);

    info!(
        n_account_tanks = by_account_tank.len(),
        "calculating account⨯tank victory ratios…"
    );
    let by_account_tank = calculate_victory_ratios(by_account_tank, z_level);

    let train_set = build_matrix(&by_vehicle, by_account_tank);

    let tank_ids = by_vehicle.keys().copied().collect_vec();
    let similarities = calculate_similarities(train_set, &tank_ids, opts.buffering).await?;
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
async fn aggregate_train_set(
    mut train_set: impl Stream<Item = Result<database::TrainItem>> + Unpin,
) -> Result<(
    AHashMap<wargaming::TankId, Sample>,
    AHashMap<(wargaming::TankId, wargaming::AccountId), Sample>,
)> {
    info!("aggregating…");

    let mut by_account_tank = AHashMap::default();
    let mut n_battles = 0;
    let start_instant = Instant::now();

    while let Some(item) = train_set.try_next().await? {
        n_battles += item.n_battles;
        let sample = Sample::from(&item);

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
    Ok((by_vehicle, by_account_tank))
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
    train_set: CscMatrix<f64>,
    tank_ids: &[wargaming::TankId],
    buffering: usize,
) -> Result<AHashMap<wargaming::TankId, Vec<(wargaming::TankId, f64)>>> {
    info!(nnz = train_set.nnz(), n_vehicles = tank_ids.len(), "calculating similarities…");
    let start_instant = Instant::now();

    let train_set = Arc::new(train_set);
    let mut stream = stream::iter(tank_ids)
        .flat_map(|tank_id_1| stream::iter(tank_ids).map(|tank_id_2| (*tank_id_1, *tank_id_2)))
        .filter(|(tank_id_1, tank_id_2)| ready(tank_id_2 < tank_id_1))
        .map(|(tank_id_1, tank_id_2)| {
            let train_set = Arc::clone(&train_set);
            spawn(async move {
                let column_1 = train_set.col(tank_id_1 as usize);
                let column_2 = train_set.col(tank_id_2 as usize);
                (tank_id_1, tank_id_2, similarity(&column_1, &column_2))
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

fn similarity(column_1: &CscCol<f64>, column_2: &CscCol<f64>) -> f64 {
    let users_1 = column_1.row_indices().iter().zip(column_1.values().iter());
    let users_2 = column_2.row_indices().iter().zip(column_2.values().iter());
    let numerator: f64 = users_1
        .merge_join_by(users_2, |(i, _), (j, _)| i.cmp(j))
        .filter_map(|item| item.both().map(|((_, x), (_, y))| (x, y)))
        .map(|(x, y)| x * y)
        .sum();
    let denominator_1: f64 = column_1.values().iter().map(|x| x * x).sum();
    let denominator_2: f64 = column_2.values().iter().map(|y| y * y).sum();
    numerator / denominator_1.sqrt() / denominator_2.sqrt()
}
