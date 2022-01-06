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

/// Comes from the Wargaming.net API limitation.
const MAX_BATCH_SIZE: usize = 100;

/// Generates an infinite stream of batches, looping through the entire account table.
pub async fn get_batch_stream(
    database: PgPool,
    min_offset: Arc<RwLock<StdDuration>>,
) -> impl Stream<Item = crate::Result<Batch>> {
    stream::try_unfold(
        (database, min_offset),
        move |(database, min_offset)| async move {
            loop {
                let batch = {
                    let min_offset = *min_offset.read().await;
                    retrieve_batch(&database, min_offset, MAX_BATCH_SIZE).await?
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
    min_offset: StdDuration,
    count: usize,
) -> crate::Result<Batch> {
    // language=SQL
    const QUERY: &str = "
        -- CREATE EXTENSION tsm_system_rows;
        SELECT account_id, last_battle_time FROM accounts TABLESAMPLE system_rows($2)
        WHERE last_battle_time < now() - $1;
    ";
    let query = sqlx::query_as(QUERY)
        .bind(min_offset)
        .bind(i32::try_from(count)?);
    timeout(StdDuration::from_secs(60), query.fetch_all(database))
        .await
        .context("the `retrieve_batch` query has timed out")?
        .context("failed to retrieve a batch")
}
