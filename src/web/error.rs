use rocket::http::Status;
use rocket::response::Responder;
use rocket::{response, Request, Response};
use sentry::integrations::anyhow::capture_anyhow;

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
        let sentry_id = capture_anyhow(&self.0).to_simple().to_string();
        log::error!(
            "{} {}: {:#} (https://sentry.io/eigenein/blitz-dashboard/events/{})",
            request.method(),
            request.uri(),
            self.0,
            sentry_id,
        );
        Response::build()
            .status(Status::InternalServerError)
            .raw_header("x-sentry-id", sentry_id)
            .ok()
    }
}
