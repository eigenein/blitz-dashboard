use poem::http::StatusCode;
use poem::{handler, IntoResponse};

use crate::prelude::*;

#[allow(dead_code)]
#[instrument(skip_all)]
#[handler]
pub async fn get() -> impl IntoResponse {
    StatusCode::GONE
}
