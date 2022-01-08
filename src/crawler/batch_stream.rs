use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::Context;
use futures::{stream, Stream};
use sqlx::PgPool;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{instrument, warn};

use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

/// Generates an infinite stream of batches, looping through the entire account table.
pub async fn get_batch_stream(
    database: PgPool,
    inner_limit: usize,
    min_offset: Arc<RwLock<StdDuration>>,
) -> impl Stream<Item = crate::Result<Batch>> {
    stream::try_unfold(
        (database, min_offset),
        move |(database, min_offset)| async move {
            loop {
                let batch = {
                    let min_offset = *min_offset.read().await;
                    retrieve_batch(&database, inner_limit, min_offset).await?
                };
                if !batch.is_empty() {
                    break Ok(Some((batch, (database, min_offset))));
                }
                warn!("no accounts matched, sleeping…");
                sleep(StdDuration::from_secs(1)).await;
            }
        },
    )
}

/// Retrieves a single account batch from the database.
#[instrument(skip_all, level = "debug")]
async fn retrieve_batch(
    database: &PgPool,
    inner_limit: usize,
    min_offset: StdDuration,
) -> crate::Result<Batch> {
    // language=SQL
    const QUERY: &str = r#"
        -- CREATE EXTENSION tsm_system_rows;
        WITH "inner" AS (
            SELECT account_id, last_battle_time
            FROM accounts TABLESAMPLE system_rows($2)
            ORDER BY random()
        )
        SELECT * FROM "inner"
        WHERE last_battle_time IS NULL OR (last_battle_time < NOW() - $1)
        LIMIT 100
    "#;
    let query = sqlx::query_as(QUERY)
        .bind(min_offset)
        .bind(inner_limit as i32);
    timeout(StdDuration::from_secs(60), query.fetch_all(database))
        .await
        .context("the `retrieve_batch` query has timed out")?
        .context("failed to retrieve a batch")
}
