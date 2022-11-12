use poem::{Endpoint, Middleware, Request, Response, Result};

use crate::prelude::*;

pub struct TimeItMiddleware;

impl<E: Endpoint<Output = Response>> Middleware<E> for TimeItMiddleware {
    type Output = TimeItMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        TimeItMiddlewareImpl { ep }
    }
}

pub struct TimeItMiddlewareImpl<E> {
    ep: E,
}

#[poem::async_trait]
impl<E: Endpoint<Output = Response>> Endpoint for TimeItMiddlewareImpl<E> {
    type Output = Response;

    async fn call(&self, request: Request) -> Result<Self::Output> {
        let method = request.method().clone();
        let uri = request.uri().clone();
        let start_instant = Instant::now();
        let response = self.ep.call(request).await;
        info!(elapsed = ?start_instant.elapsed(), ?method, ?uri);
        response
    }
}
