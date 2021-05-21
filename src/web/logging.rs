use std::time::Instant;

pub struct RequestLogMiddleware;

struct RequestLogMiddlewareHasBeenRun;

#[tide::utils::async_trait]
impl<T: Clone + Send + Sync + 'static> tide::Middleware<T> for RequestLogMiddleware {
    async fn handle(&self, mut request: tide::Request<T>, next: tide::Next<'_, T>) -> tide::Result {
        if request.ext::<RequestLogMiddlewareHasBeenRun>().is_some() {
            return Ok(next.run(request).await);
        }
        request.set_ext(RequestLogMiddlewareHasBeenRun);

        let peer_addr = request.peer_addr().unwrap_or("-").to_string();
        let path = request.url().path().to_string();
        let method = request.method().to_string();
        let start = Instant::now();
        let response = next.run(request).await;
        let duration = Instant::now() - start;
        if let Some(error) = response.error() {
            log::error!("Error processing the request: {:?}", error);
        }
        log::info!(
            r#"Request: {peer_addr} {method} {path} {status} ({duration:#?})"#,
            peer_addr = peer_addr,
            method = method,
            path = path,
            status = response.status(),
            duration = duration,
        );
        Ok(response)
    }
}
