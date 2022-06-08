use rocket::http::Status;
use rocket::response::Responder;
use rocket::{response, Request, Response};
use tracing::error;

/// [`anyhow::Error`] wrapper that allows to use the `?` operator in routes.
#[derive(Debug)]
pub struct Error(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for Error {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        error!("{} {}: {:#}", request.method(), request.uri(), self.0);
        Response::build().status(Status::InternalServerError).ok()
    }
}
