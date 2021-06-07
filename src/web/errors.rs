use tide::{Request, StatusCode};

use crate::web::state::State;

/// Debug endpoint that always returns an error.
pub async fn get(_request: Request<State>) -> tide::Result {
    Err(tide::Error::from_str(
        StatusCode::InternalServerError,
        "Simulated error",
    ))
}
