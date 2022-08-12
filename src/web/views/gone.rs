use poem::http::StatusCode;
use poem::{handler, IntoResponse};

use crate::prelude::*;

#[instrument(skip_all)]
#[handler]
pub async fn get() -> impl IntoResponse {
    StatusCode::GONE
}
