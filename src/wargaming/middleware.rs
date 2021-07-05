use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration as StdDuration, Instant};

use async_std::future::timeout;
use clap::{crate_name, crate_version};
use surf::middleware::{Middleware, Next};
use surf::{Client, Request};

static COUNTER: AtomicU32 = AtomicU32::new(1);

#[derive(Debug)]
pub struct UserAgent;

#[derive(Debug)]
pub struct TimeoutAndRetry {
    pub timeout: StdDuration,
    pub n_retries: i32,
}

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
impl Middleware for TimeoutAndRetry {
    async fn handle(&self, request: Request, client: Client, next: Next<'_>) -> surf::Result {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let request = request.clone();
            let client = client.clone();
            match timeout(self.timeout, next.run(request, client)).await {
                Ok(result) => break result,
                Err(error) if attempt == self.n_retries => {
                    break Err(anyhow::Error::new(error)
                        .context(format!("The API has timed out {} times", attempt))
                        .into());
                }
                Err(error) => {
                    log::warn!("The API has timed out: {:#}. Retrying…", error);
                }
            }
        }
    }
}

#[surf::utils::async_trait]
impl Middleware for Logger {
    async fn handle(&self, request: Request, client: Client, next: Next<'_>) -> surf::Result {
        let start_instant = Instant::now();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        log::debug!("{} #{} → {}", request.method(), id, request.url());
        let response = next.run(request, client).await?;
        let time_elapsed = Instant::now() - start_instant;
        log::debug!(
            "#{} → [{}] ({:#?}): {:?} bytes",
            id,
            response.status(),
            time_elapsed,
            response.len(),
        );
        Ok(response)
    }
}
