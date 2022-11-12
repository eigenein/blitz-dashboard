use poem::http::StatusCode;
use poem::{handler, IntoResponse, Response};

use crate::prelude::*;

const CACHE_CONTROL: &str = "no-cache";

#[handler]
#[instrument(skip_all, level = "info")]
pub async fn get_health() -> Result<impl IntoResponse> {
    Ok(Response::from(StatusCode::NO_CONTENT).with_header("Cache-Control", CACHE_CONTROL))
}
