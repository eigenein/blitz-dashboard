use std::collections::BTreeMap;

use poem::{Endpoint, Middleware, Request, Result};

pub struct SentryMiddleware;

impl<E: Endpoint> Middleware<E> for SentryMiddleware {
    type Output = SentryMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SentryMiddlewareImpl { ep }
    }
}

pub struct SentryMiddlewareImpl<E> {
    ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for SentryMiddlewareImpl<E> {
    type Output = E::Output;

    async fn call(&self, request: Request) -> Result<Self::Output> {
        sentry::configure_scope(|scope| {
            scope.set_tag("request.method", request.method().as_str());
            scope.set_tag("request.path", request.uri().path());
            scope.set_tag("request.remote_addr", request.remote_addr());

            let mut context = BTreeMap::new();
            context.insert("query".to_string(), request.uri().query().into());
            scope.set_context("request", sentry::protocol::Context::Other(context));
        });
        self.ep.call(request).await
    }
}
