use std::borrow::Cow;
use std::time::{Duration, Instant};

use log::{debug, error, log, Level};
use sentry::integrations::anyhow::capture_anyhow;
use tide::{Middleware, Request};

pub struct LoggerMiddleware;
pub struct SecurityMiddleware;

impl LoggerMiddleware {
    const REQUEST_DURATION_THRESHOLD: Duration = Duration::from_secs(1);
}

#[tide::utils::async_trait]
impl<T: Clone + Send + Sync + 'static> Middleware<T> for LoggerMiddleware {
    async fn handle(&self, request: Request<T>, next: tide::Next<'_, T>) -> tide::Result {
        let peer_address =
            get_peer_address(&request).map_or_else(|| Cow::Borrowed("-"), Cow::Owned);
        let path = request.url().path().to_string();
        let method = request.method().to_string();
        debug!("{} → {} {}", peer_address, method, path);
        let start = Instant::now();
        let mut response = next.run(request).await;
        let duration = Instant::now() - start;
        let level = if response.status().is_client_error()
            || duration >= Self::REQUEST_DURATION_THRESHOLD
        {
            Level::Warn
        } else {
            Level::Info
        };
        log!(
            level,
            r#"{peer_address} ← {method} {path} [{status}] ({duration:#?})"#,
            peer_address = peer_address,
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
                Ok(crate::web::responses::error(&sentry_id))
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

pub fn get_peer_address<T>(request: &Request<T>) -> Option<String> {
    request
        .header("X-Real-IP")
        .or_else(|| request.header("X-Forwarded-For"))
        .map(|values| values.as_str())
        .or_else(|| request.peer_addr())
        .map(ToString::to_string)
}
