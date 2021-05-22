use crate::web::State;
use tide::{Request, StatusCode};

/// Debug endpoint that always returns an error.
pub async fn get(_request: Request<State>) -> tide::Result {
    Err(tide::Error::from_str(
        StatusCode::InternalServerError,
        "This is a simulated error",
    ))
}
