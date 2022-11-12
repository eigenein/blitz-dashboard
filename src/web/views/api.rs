use futures::StreamExt;
use poem::http::StatusCode;
use poem::web::{Data, Path};
use poem::{handler, Body, IntoResponse, Response};

use crate::database::AccountIdProjection;
use crate::prelude::*;

const CACHE_CONTROL: &str = "no-cache";

#[handler]
#[instrument(skip_all, level = "info")]
pub async fn get_health() -> Result<impl IntoResponse> {
    Ok(Response::from(StatusCode::NO_CONTENT).with_header("Cache-Control", CACHE_CONTROL))
}

#[handler]
#[instrument(skip_all, level = "info")]
pub async fn get_active_since(
    db: Data<&mongodb::Database>,
    Path((realm, since)): Path<(wargaming::Realm, DateTime)>,
) -> Result<impl IntoResponse> {
    let stream = AccountIdProjection::retrieve_active_since(&db, realm, since)
        .await?
        .map(move |account| {
            account.map(|account| format!("{{\"id\":{}}}\n", account.id).into_bytes())
        });
    Ok(Response::from(Body::from_bytes_stream(stream))
        .with_header("Cache-Control", CACHE_CONTROL)
        .with_content_type("application/json"))
}
