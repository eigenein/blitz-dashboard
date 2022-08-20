use bpci::{Interval, NSuccessesSample, WilsonScore};
use futures::{Stream, TryStreamExt};
use nalgebra_sparse::{CooMatrix, CscMatrix};

use crate::database::mongodb::traits::TypedDocument;
use crate::math::statistics::ConfidenceLevel;
use crate::opts::TrainOpts;
use crate::prelude::*;
use crate::{database, wargaming};

pub async fn run(opts: TrainOpts) -> Result {
    let db = database::mongodb::open(&opts.connections.mongodb_uri).await?;
    let deadline = now() - Duration::from_std(opts.train_period)?;

    info!(?deadline, "querying the vehicle stats…");
    let vehicle_stats = database::VehicleStats::retrieve_all(&db, opts.realm, deadline).await?;
    info!(n_vehicles = vehicle_stats.len());

    let estimated_document_count = database::TrainItem::collection(&db)
        .estimated_document_count(None)
        .await?;
    info!(estimated_document_count, "querying the train set…");

    let train_set = database::TrainItem::retrieve_all(&db, opts.realm, deadline).await?;
    let train_set = read_matrix(&vehicle_stats, train_set, opts.confidence_level).await?;
    info!(
        nnz = train_set.nnz(),
        proportion = (train_set.nnz() as f64 / estimated_document_count as f64),
        "converting to CSC…",
    );
    let train_set = CscMatrix::from(&train_set);

    Ok(())
}

#[instrument(level = "info", skip_all, fields(confidence_level = ?confidence_level))]
async fn read_matrix(
    vehicle_stats: &AHashMap<wargaming::TankId, database::VehicleStats>,
    mut train_set: impl Stream<Item = Result<database::TrainItem>> + Unpin,
    confidence_level: ConfidenceLevel,
) -> Result<CooMatrix<f64>> {
    let mut matrix = CooMatrix::new(u32::MAX as usize, u16::MAX as usize);
    let z_level = confidence_level.z_value();
    while let Some(item) = train_set.try_next().await? {
        if let Some(vehicle_stats) = vehicle_stats.get(&item.tank_id) {
            let interval =
                NSuccessesSample::new(item.n_battles, item.n_wins)?.wilson_score_with_cc(z_level);
            if !interval.contains(vehicle_stats.victory_ratio) {
                matrix.push(item.account_id as usize, item.tank_id as usize, interval.mean());
            }
        }
    }
    info!(nnz = matrix.nnz(), "constructed the CSC matrix");
    Ok(matrix)
}
