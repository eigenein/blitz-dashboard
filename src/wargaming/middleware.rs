use std::borrow::Cow;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use async_std::future::timeout;
use clap::{crate_name, crate_version};
use surf::middleware::{Middleware, Next};
use surf::{Body, Client, Request};

static COUNTER: AtomicU32 = AtomicU32::new(1);

#[derive(Debug)]
pub struct UserAgent;

#[derive(Debug)]
pub struct Timeout(pub Duration);

#[derive(Debug)]
pub struct Logger;

#[surf::utils::async_trait]
impl Middleware for UserAgent {
    async fn handle(&self, mut request: Request, client: Client, next: Next<'_>) -> surf::Result {
        request.set_header("User-Agent", concat!(crate_name!(), "/", crate_version!()));
        next.run(request, client).await
    }
}

#[surf::utils::async_trait]
impl Middleware for Timeout {
    async fn handle(&self, request: Request, client: Client, next: Next<'_>) -> surf::Result {
        timeout(self.0, next.run(request, client)).await?
    }
}

#[surf::utils::async_trait]
impl Middleware for Logger {
    async fn handle(&self, request: Request, client: Client, next: Next<'_>) -> surf::Result {
        let start_instant = Instant::now();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        log::debug!("{} #{} → {}", request.method(), id, request.url());
        let mut response = next.run(request, client).await?;
        let time_elapsed = Instant::now() - start_instant;
        let body = response.take_body().into_string().await?;
        log::debug!(
            "#{} → [{}] ({:#?}): {}",
            id,
            response.status(),
            time_elapsed,
            if body.len() < 100 {
                Cow::Borrowed(&body)
            } else {
                Cow::Owned(format!("[{} chars]", body.len()))
            }
        );
        response.set_body(Body::from(body));
        Ok(response)
    }
}
