use poem::http::HeaderValue;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

pub struct SecurityHeadersMiddleware;

impl<E: Endpoint> Middleware<E> for SecurityHeadersMiddleware {
    type Output = SecurityHeadersMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SecurityHeadersMiddlewareImpl { ep }
    }
}

pub struct SecurityHeadersMiddlewareImpl<E> {
    ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for SecurityHeadersMiddlewareImpl<E> {
    type Output = Response;

    async fn call(&self, request: Request) -> Result<Self::Output> {
        let mut response = self.ep.call(request).await?.into_response();
        let headers = response.headers_mut();
        headers.remove("Server");
        headers.append("X-DNS-Prefetch-Control", HeaderValue::from_static("on"));
        headers.append("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
        headers.append("X-Frame-Options", HeaderValue::from_static("deny"));
        headers.append("Strict-Transport-Security", HeaderValue::from_static("max-age=5184000"));
        headers.append("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
        Ok(response)
    }
}
