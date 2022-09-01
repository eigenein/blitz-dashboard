use std::net::IpAddr;
use std::str::FromStr;

use poem::http::StatusCode;
use poem::listener::TcpListener;
use poem::middleware::{CatchPanic, Tracing};
use poem::web::{Data, Json};
use poem::{handler, post, EndpointExt, IntoResponse, Response, Route, Server};
use tokio::sync::RwLock;

use crate::math::{logit, sigmoid};
use crate::prelude::*;
use crate::trainer::model::Model;
use crate::trainer::requests::RecommendRequest;
use crate::trainer::sample::Sample;
use crate::web::middleware::{ErrorMiddleware, SecurityHeadersMiddleware, SentryMiddleware};

pub async fn run(host: &str, port: u16, model: Arc<RwLock<Model>>) -> Result {
    let app = Route::new()
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
#[instrument(level = "info", skip_all)]
async fn recommend(
    Json(request): Json<RecommendRequest>,
    Data(model): Data<&Arc<RwLock<Model>>>,
) -> Result<Response> {
    let start_instant = Instant::now();
    debug!(?request.realm, n_given = request.given.len(), n_predict = ?request.predict.len());

    let model = model.read().await;
    let regressions = match model.regressions.get(&request.realm) {
        Some(regressions) => regressions,
        _ => {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };

    let mut predictions = Vec::<(wargaming::TankId, f64)>::new();
    for target_vehicle_id in request.predict {
        let regressions = match regressions.get(&target_vehicle_id) {
            Some(regressions) => regressions,
            _ => {
                continue;
            }
        };
        let mut prediction_sum = 0.0;
        let mut total_weight = 0.0;
        for given in &request.given {
            if let Some(regression) = regressions.get(&given.tank_id) {
                let source_weight =
                    (given.sample.n_battles + Sample::PRIOR_ALPHA + Sample::PRIOR_BETA) as f64;
                prediction_sum +=
                    sigmoid(regression.predict(logit(given.sample.mean()))) * source_weight;
                total_weight += source_weight;
            }
        }
        if total_weight != 0.0 {
            predictions.push((target_vehicle_id, prediction_sum / total_weight));
        }
    }
    predictions.sort_unstable_by(|(_, lhs), (_, rhs)| rhs.total_cmp(lhs));

    info!(?request.realm, n_predictions = predictions.len(), elapsed = ?start_instant.elapsed());
    Ok(Json(predictions).into_response())
}
