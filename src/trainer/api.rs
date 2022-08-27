use std::net::IpAddr;
use std::str::FromStr;

use futures::{stream, StreamExt};
use poem::http::StatusCode;
use poem::listener::TcpListener;
use poem::middleware::{CatchPanic, Tracing};
use poem::web::{Data, Json, Path};
use poem::{get, handler, post, EndpointExt, IntoResponse, Response, Route, Server};
use tokio::sync::RwLock;

use crate::prelude::*;
use crate::trainer::requests::RecommendRequest;
use crate::trainer::responses;
use crate::web::middleware::{ErrorMiddleware, SecurityHeadersMiddleware, SentryMiddleware};

pub async fn run(
    host: &str,
    port: u16,
    vehicle_victory_ratios: Arc<RwLock<AHashMap<wargaming::TankId, f64>>>,
    vehicle_similarities: Arc<RwLock<AHashMap<(wargaming::TankId, wargaming::TankId), f64>>>,
) -> Result {
    let app = Route::new()
        .at("/vehicles/:vehicle_id", get(get_vehicle))
        .at("/recommend", post(recommend))
        .data(vehicle_victory_ratios)
        .data(vehicle_similarities)
        .with(Tracing)
        .with(CatchPanic::new())
        .with(ErrorMiddleware)
        .with(SecurityHeadersMiddleware)
        .with(SentryMiddleware);
    Server::new(TcpListener::bind((IpAddr::from_str(host)?, port)))
        .run(app)
        .await?;
    Ok(())
}

#[handler]
#[instrument(level = "info", skip_all, fields(tank_id = tank_id))]
async fn get_vehicle(
    Path(tank_id): Path<wargaming::TankId>,
    Data(vehicle_victory_ratios): Data<&Arc<RwLock<AHashMap<wargaming::TankId, f64>>>>,
    Data(vehicle_similarities): Data<
        &Arc<RwLock<AHashMap<(wargaming::TankId, wargaming::TankId), f64>>>,
    >,
) -> Result<Response> {
    let start_instant = Instant::now();
    let victory_ratio = match vehicle_victory_ratios.read().await.get(&tank_id) {
        Some(victory_ratio) => *victory_ratio,
        _ => {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };
    let tank_ids = vehicle_victory_ratios
        .read()
        .await
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let mut response = responses::VehicleResponse {
        victory_ratio,
        similar_vehicles: stream::iter(tank_ids)
            .filter_map(|target_id| async move {
                vehicle_similarities
                    .read()
                    .await
                    .get(&(tank_id, target_id))
                    .map(|similarity| (target_id, *similarity))
            })
            .collect::<Vec<(wargaming::TankId, f64)>>()
            .await,
    };
    response
        .similar_vehicles
        .sort_unstable_by(|(_, lhs), (_, rhs)| rhs.total_cmp(lhs));
    info!(elapsed = ?start_instant.elapsed(), "completed");
    Ok(Json(response).into_response())
}

#[handler]
#[instrument(level = "info", skip_all)]
async fn recommend(
    Json(request): Json<RecommendRequest>,
    Data(vehicle_victory_ratios): Data<&Arc<RwLock<AHashMap<wargaming::TankId, f64>>>>,
    Data(vehicle_similarities): Data<
        &Arc<RwLock<AHashMap<(wargaming::TankId, wargaming::TankId), f64>>>,
    >,
) -> Result<Json<Vec<(wargaming::TankId, f64)>>> {
    let start_instant = Instant::now();
    info!(n_given = request.given.len(), n_predict = ?request.predict.len());
    let given = {
        stream::iter(request.given.into_iter())
            .filter_map(|(tank_id, victory_ratio)| async move {
                vehicle_victory_ratios
                    .read()
                    .await
                    .get(&tank_id)
                    .map(|vehicle_victory_ratio| (tank_id, victory_ratio - vehicle_victory_ratio))
            })
            .collect::<Vec<_>>()
            .await
    };
    let given = Arc::new(given);
    let mut predictions = {
        stream::iter(request.predict.into_iter())
            .filter_map(|target_id| {
                let given = Arc::clone(&given);
                async move {
                    let vehicle_similarities = vehicle_similarities.read().await;
                    vehicle_victory_ratios
                        .read()
                        .await
                        .get(&target_id)
                        .and_then(|target_victory_ratio| {
                            let mut numerator = 0.0;
                            let mut denominator = 0.0;
                            for (source_id, rating) in given.iter() {
                                if let Some(similarity) =
                                    vehicle_similarities.get(&(*source_id, target_id))
                                {
                                    numerator += rating * similarity;
                                    denominator += similarity;
                                }
                            }
                            let target_rating = numerator / denominator;
                            target_rating
                                .is_finite()
                                .then(|| (target_id, target_rating + target_victory_ratio))
                        })
                }
            })
            .collect::<Vec<_>>()
            .await
    };
    predictions.sort_unstable_by(|(_, lhs), (_, rhs)| rhs.total_cmp(lhs));
    info!(n_predictions = predictions.len(), elapsed = ?start_instant.elapsed());
    Ok(Json(predictions))
}
