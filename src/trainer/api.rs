use std::net::IpAddr;
use std::str::FromStr;

use poem::http::StatusCode;
use poem::listener::TcpListener;
use poem::middleware::{CatchPanic, Tracing};
use poem::web::{Data, Json, Path};
use poem::{get, handler, post, EndpointExt, IntoResponse, Response, Route, Server};
use tokio::sync::RwLock;

use crate::math::{logit, sigmoid};
use crate::prelude::*;
use crate::trainer::model::Model;
use crate::trainer::requests::RecommendRequest;
use crate::trainer::responses::{Prediction, RecommendResponse};
use crate::web::middleware::{ErrorMiddleware, SecurityHeadersMiddleware, SentryMiddleware};

pub async fn run(host: &str, port: u16, model: Arc<RwLock<Model>>) -> Result {
    let app = Route::new()
        .at("/recommend", post(recommend))
        .at("/:realm/:source_id/:target_id/regression", get(get_regression))
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
#[instrument(level = "info", skip_all)]
async fn recommend(
    Json(request): Json<RecommendRequest>,
    Data(model): Data<&Arc<RwLock<Model>>>,
) -> Result<Json<RecommendResponse>> {
    let start_instant = Instant::now();
    debug!(?request.realm, n_given = request.given.len(), n_predict = ?request.predict.len());

    let model = model.read().await;
    let regressions = match model.regressions.get(&request.realm) {
        Some(regressions) => regressions,
        _ => {
            return Ok(Json(RecommendResponse::default()));
        }
    };

    let mut predictions = Vec::<Prediction>::new();
    for target_vehicle_id in request.predict {
        let regressions = match regressions.get(&target_vehicle_id) {
            Some(regressions) => regressions,
            _ => {
                continue;
            }
        };

        let mut total_weight = 0.0;
        let mut prediction = Prediction::new(target_vehicle_id);

        for given in &request.given {
            let source_weight = given.sample.n_posterior_battles_f64();
            if let Some(regression) = regressions.get(&given.tank_id) {
                prediction.p += sigmoid(regression.predict(logit(given.sample.posterior_mean())))
                    * source_weight;
                total_weight += source_weight;
                prediction.n_sources += 1;
            }
        }
        if total_weight != 0.0 {
            prediction.p /= total_weight;
            if prediction.p >= request.min_prediction {
                predictions.push(prediction);
            }
        }
    }
    predictions.sort_unstable();
    predictions.reverse();

    info!(?request.realm, n_predictions = predictions.len(), elapsed = ?start_instant.elapsed());
    Ok(Json(RecommendResponse { predictions }))
}

#[handler]
#[instrument(level = "info", skip_all)]
async fn get_regression(
    Path((realm, source_vehicle_id, target_vehicle_id)): Path<(
        wargaming::Realm,
        wargaming::TankId,
        wargaming::TankId,
    )>,
    Data(model): Data<&Arc<RwLock<Model>>>,
) -> Response {
    let model = model.read().await;
    let realm_regressions = match model.regressions.get(&realm) {
        Some(realm_regressions) => realm_regressions,
        _ => {
            return StatusCode::NOT_FOUND.into_response();
        }
    };
    let target_regressions = match realm_regressions.get(&target_vehicle_id) {
        Some(target_regressions) => target_regressions,
        _ => {
            return StatusCode::NOT_FOUND.into_response();
        }
    };
    let regression = match target_regressions.get(&source_vehicle_id) {
        Some(regression) => regression,
        _ => {
            return StatusCode::NOT_FOUND.into_response();
        }
    };
    Json(regression).into_response()
}
