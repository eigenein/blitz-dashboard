use std::time::Instant;

use log::{debug, error, log, Level};
use sentry::integrations::anyhow::capture_anyhow;
use tide::{Middleware, Request};

pub struct LoggerMiddleware;
pub struct SecurityMiddleware;

#[tide::utils::async_trait]
impl<T: Clone + Send + Sync + 'static> Middleware<T> for LoggerMiddleware {
    async fn handle(&self, request: Request<T>, next: tide::Next<'_, T>) -> tide::Result {
        let peer_addr = request.peer_addr().unwrap_or("-").to_string();
        let path = request.url().path().to_string();
        let method = request.method().to_string();
        debug!("{} → {} {}", peer_addr, method, path);
        let start = Instant::now();
        let mut response = next.run(request).await;
        let duration = Instant::now() - start;
        let level = if response.status().is_client_error() {
            Level::Warn
        } else {
            Level::Info
        };
        log!(
            level,
            r#"{peer_addr} ← {method} {path} [{status}] ({duration:#?})"#,
            peer_addr = peer_addr,
            method = method,
            path = path,
            status = response.status(),
            duration = duration,
        );
        match response.take_error() {
            Some(error) => {
                let error = error.into_inner();
                let sentry_id = capture_anyhow(&error);
                error!(
                    "{} {}: {:#} (https://sentry.io/eigenein/blitz-dashboard/events/{})",
                    method,
                    path,
                    error,
                    sentry_id.to_simple()
                );
                Ok(crate::web::responses::render_error(&sentry_id))
            }
            None => Ok(response),
        }
    }
}

#[tide::utils::async_trait]
impl<T: Clone + Send + Sync + 'static> Middleware<T> for SecurityMiddleware {
    async fn handle(&self, request: Request<T>, next: tide::Next<'_, T>) -> tide::Result {
        let mut response = next.run(request).await;
        http_types::security::default(&mut response);
        Ok(response)
    }
}
