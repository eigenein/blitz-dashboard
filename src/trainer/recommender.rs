use ahash::AHashMap;
use bpci::Interval;
use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::Database;

use crate::database::mongodb::traits::TypedDocument;
use crate::math::statistics::ConfidenceLevel;
use crate::math::traits::TrueWinRate;
use crate::prelude::*;

#[instrument(level = "info", skip_all)]
pub async fn recommend(
    db: &Database,
    all_tank_ids: &[wargaming::TankId],
    stats_delta: &[database::TankSnapshot],
    confidence_level: ConfidenceLevel,
) -> Result<Vec<(wargaming::TankId, f64)>> {
    if stats_delta.is_empty() {
        return Ok(Vec::new());
    }

    info!(n_source_vehicles = stats_delta.len(), n_target_vehicles = all_tank_ids.len());
    let start_instant = Instant::now();
    let vehicle_models = database::VehicleModel::collection(db)
        .find(doc! { "_id": { "$in": all_tank_ids } }, None)
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    info!(elapsed = ?start_instant.elapsed(), "vehicle models collected");
    let mut ratings = stats_delta
        .iter()
        .map(|snapshot| {
            Ok((snapshot.tank_id, snapshot.stats.true_win_rate(confidence_level)?.mean()))
        })
        .collect::<Result<AHashMap<_, _>>>()?;
    for model in &vehicle_models {
        if let Some(rating) = ratings.get_mut(&model.tank_id) {
            *rating -= model.victory_ratio;
        }
    }
    let ratings = ratings;
    let mut predictions = vehicle_models
        .into_iter()
        .filter_map(|target_model| {
            let (numerator, denominator) = target_model
                .similar
                .into_iter()
                .filter_map(|source_vehicle| {
                    ratings
                        .get(&source_vehicle.tank_id)
                        .map(|rating| (source_vehicle.similarity, *rating))
                })
                .fold((0.0, 0.0), |(numerator, denominator), (similarity, rating)| {
                    (numerator + similarity * rating, denominator + similarity)
                });
            let prediction = numerator / denominator;
            prediction
                .is_finite()
                .then(|| (target_model.tank_id, prediction + target_model.victory_ratio))
        })
        .collect::<Vec<_>>();
    predictions.sort_unstable_by(|(_, lhs), (_, rhs)| rhs.total_cmp(lhs));
    info!(elapsed = ?start_instant.elapsed(), "finished");
    Ok(predictions)
}
