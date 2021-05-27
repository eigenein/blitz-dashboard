use sentry::capture_error;
use std::time::Instant;
use tide::Request;

pub struct LoggerMiddleware;

#[tide::utils::async_trait]
impl<T: Clone + Send + Sync + 'static> tide::Middleware<T> for LoggerMiddleware {
    async fn handle(&self, request: Request<T>, next: tide::Next<'_, T>) -> tide::Result {
        let peer_addr = request.peer_addr().unwrap_or("-").to_string();
        let path = request.url().path().to_string();
        let method = request.method().to_string();
        log::info!("Begin request: {} {} {}", peer_addr, method, path);
        let start = Instant::now();
        let response = next.run(request).await;
        let duration = Instant::now() - start;
        log::info!(
            r#"Request: {peer_addr} {method} {path} [{status}] ({duration:#?})"#,
            peer_addr = peer_addr,
            method = method,
            path = path,
            status = response.status(),
            duration = duration,
        );
        match response.error() {
            Some(error) => {
                let sentry_id = capture_error::<dyn std::error::Error>(error.as_ref());
                log::error!("Response error: {:?} [{}]", error, sentry_id.to_simple());
                Ok(crate::web::responses::render_error(&sentry_id))
            }
            None => Ok(response),
        }
    }
}
