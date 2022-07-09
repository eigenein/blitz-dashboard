use std::collections::BTreeMap;

use poem::web::RealIp;
use poem::{Endpoint, FromRequest, Middleware, Request, Result};

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
        let real_ip = RealIp::from_request(&request, &mut Default::default())
            .await?
            .0;
        let request_context = BTreeMap::from([("query".to_string(), request.uri().query().into())]);

        sentry::configure_scope(|scope| {
            scope.set_tag("request.method", request.method().as_str());
            scope.set_tag("request.path", request.uri().path());
            if let Some(real_ip) = real_ip {
                scope.set_tag("request.real_ip", real_ip);
            }
            scope.set_context("request", sentry::protocol::Context::Other(request_context));
        });

        self.ep.call(request).await
    }
}
