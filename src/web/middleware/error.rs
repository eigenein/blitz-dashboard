use poem::http::StatusCode;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

use crate::prelude::*;

pub struct ErrorMiddleware;

impl<E: Endpoint> Middleware<E> for ErrorMiddleware {
    type Output = ErrorMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ErrorMiddlewareImpl { ep }
    }
}

pub struct ErrorMiddlewareImpl<E> {
    ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ErrorMiddlewareImpl<E> {
    type Output = Response;

    async fn call(&self, request: Request) -> Result<Self::Output> {
        let method = request.method().clone();
        let uri = request.uri().clone();
        match self.ep.call(request).await {
            Ok(response) => {
                let response = response.into_response();
                if response.status().is_client_error() {
                    info!(?method, ?uri, status = ?response.status(), "client error");
                }
                if response.status().is_client_error() {
                    error!(?method, ?uri, status = ?response.status(), "internal server error");
                }
                Ok(response)
            }
            Err(error) => {
                error!(?method, ?uri, "{}", error);
                Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        }
    }
}
