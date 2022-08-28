use std::net::IpAddr;
use std::str::FromStr;

use futures::{stream, StreamExt};
use itertools::Itertools;
use poem::http::StatusCode;
use poem::listener::TcpListener;
use poem::middleware::{CatchPanic, Tracing};
use poem::web::{Data, Json, Path};
use poem::{get, handler, post, EndpointExt, IntoResponse, Response, Route, Server};
use tokio::sync::RwLock;

use crate::prelude::*;
use crate::trainer::model::Model;
use crate::trainer::requests::RecommendRequest;
use crate::trainer::responses;
use crate::web::middleware::{ErrorMiddleware, SecurityHeadersMiddleware, SentryMiddleware};

pub async fn run(host: &str, port: u16, model: Arc<RwLock<Model>>) -> Result {
    let app = Route::new()
        .at("/vehicles/:vehicle_id", get(get_vehicle))
        .at("/recommend", post(recommend))
        .data(model)
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
    Data(model): Data<&Arc<RwLock<Model>>>,
) -> Result<Response> {
    let start_instant = Instant::now();
    let model = model.read().await;
    let vehicle_model = match model.vehicles.get(&tank_id) {
        Some(vehicle_model) => vehicle_model,
        _ => {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };
    let response = responses::VehicleResponse {
        mean_rating: vehicle_model.mean_rating,
        similar_vehicles: vehicle_model
            .similarities
            .iter()
            .map(|(tank_id, similarity)| (*tank_id, *similarity))
            .sorted_unstable_by(|(_, lhs), (_, rhs)| rhs.total_cmp(lhs))
            .collect::<Vec<_>>(),
    };
    info!(elapsed = ?start_instant.elapsed(), "completed");
    Ok(Json(response).into_response())
}

#[handler]
#[instrument(level = "info", skip_all)]
async fn recommend(
    Json(request): Json<RecommendRequest>,
    Data(model): Data<&Arc<RwLock<Model>>>,
) -> Result<Json<Vec<(wargaming::TankId, f64)>>> {
    let start_instant = Instant::now();
    info!(n_given = request.given.len(), n_predict = ?request.predict.len());
    let given = {
        stream::iter(request.given.into_iter())
            .filter_map(|(tank_id, victory_ratio)| async move {
                model
                    .read()
                    .await
                    .vehicles
                    .get(&tank_id)
                    .map(|vehicle_model| (tank_id, victory_ratio - vehicle_model.mean_rating))
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
                    let model = model.read().await;
                    model.vehicles.get(&target_id).and_then(|vehicle_model| {
                        let mut numerator = 0.0;
                        let mut denominator = 0.0;
                        for (source_id, rating) in given.iter() {
                            if let Some(similarity) = vehicle_model.similarities.get(source_id) {
                                numerator += rating * similarity;
                                denominator += similarity;
                            }
                        }
                        let offset = numerator / denominator;
                        offset
                            .is_finite()
                            .then(|| (target_id, offset + vehicle_model.mean_rating))
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
