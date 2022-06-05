use std::time::Duration as StdDuration;

use anyhow::Context;
use futures::{stream, Stream};
use sqlx::PgPool;
use tokio::time::{sleep, timeout};
use tracing::{instrument, warn};

use crate::models::BaseAccountInfo;
use crate::prelude::*;

pub type Batch = Vec<BaseAccountInfo>;

/// Generates an infinite stream of batches, looping through the entire account table.
pub async fn get_batch_stream(
    database: PgPool,
    inner_limit: usize,
    max_offset: StdDuration,
) -> impl Stream<Item = Result<Batch>> {
    stream::try_unfold(database, move |database| async move {
        loop {
            let batch = { retrieve_batch(&database, inner_limit, max_offset).await? };
            if !batch.is_empty() {
                tracing::info!(n_accounts = batch.len(), "retrieved");
                break Ok(Some((batch, database)));
            }
            warn!("no accounts matched, sleeping…");
            sleep(StdDuration::from_secs(1)).await;
        }
    })
}

/// Retrieves a single account batch from the database.
#[instrument(skip_all, fields(inner_limit, ?max_offset))]
async fn retrieve_batch(
    database: &PgPool,
    inner_limit: usize,
    max_offset: StdDuration,
) -> Result<Batch> {
    // language=SQL
    const QUERY: &str = r#"
        -- CREATE EXTENSION tsm_system_rows;
        SELECT account_id, last_battle_time
        FROM accounts TABLESAMPLE system_rows($1)
        WHERE last_battle_time IS NULL OR (last_battle_time >= NOW() - $2)
        LIMIT 100
    "#;
    let query = sqlx::query_as(QUERY)
        .bind(inner_limit as i32)
        .bind(max_offset);
    timeout(StdDuration::from_secs(60), query.fetch_all(database))
        .await
        .context("the `retrieve_batch` query has timed out")?
        .context("failed to retrieve a batch")
}
