use async_std::future::timeout;
use clap::{crate_name, crate_version};
use surf::middleware::{Middleware, Next};
use surf::{Client, Request};

#[derive(Debug)]
pub struct UserAgent;

#[derive(Debug)]
pub struct Timeout(pub std::time::Duration);

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
