use poem::http::HeaderValue;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

pub struct SecurityHeaders;

impl<E: Endpoint> Middleware<E> for SecurityHeaders {
    type Output = SecurityHeadersImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SecurityHeadersImpl { ep }
    }
}

pub struct SecurityHeadersImpl<E> {
    ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for SecurityHeadersImpl<E> {
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
