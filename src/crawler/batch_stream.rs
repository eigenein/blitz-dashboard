use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use arc_swap::ArcSwap;
use futures::{stream, Stream};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use sqlx::PgPool;
use tokio::time::{sleep, timeout};

use crate::helpers::format_elapsed;
use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

const POINTER_KEY: &str = "crawler::pointer";

/// Generates an infinite stream of batches, looping through the entire account table.
pub async fn get_batch_stream(
    database: PgPool,
    mut redis: MultiplexedConnection,
    min_offset: Arc<ArcSwap<StdDuration>>,
) -> impl Stream<Item = crate::Result<Batch>> {
    let start_account_id = match redis.get::<_, Option<i32>>(POINTER_KEY).await {
        Ok(pointer) => pointer.unwrap_or(0),
        Err(error) => {
            tracing::error!("failed to retrieve the pointer: {:#}", error);
            0
        }
    };
    stream::try_unfold(
        (start_account_id, Instant::now()),
        move |(mut pointer, mut start_time)| {
            let database = database.clone();
            let mut redis = redis.clone();
            let min_offset = Arc::clone(&min_offset);
            async move {
                loop {
                    let batch = retrieve_batch(&database, pointer, **min_offset.load()).await?;
                    match batch.last() {
                        Some(last_item) => {
                            let pointer = last_item.id;
                            if let Err::<(), _>(error) = redis.set(POINTER_KEY, pointer).await {
                                tracing::error!(
                                    pointer = pointer,
                                    "failed to store the pointer {:#}",
                                    error,
                                );
                            }
                            break Ok(Some((batch, (pointer, start_time))));
                        }
                        None => {
                            tracing::info!(
                                elapsed = %format_elapsed(&start_time),
                                "restarting",
                            );
                            sleep(StdDuration::from_secs(1)).await;
                            start_time = Instant::now();
                            pointer = 0;
                        }
                    }
                }
            }
        },
    )
}

/// Retrieves a single account batch from the database.
#[tracing::instrument(skip_all, level = "debug", fields(starting_at = starting_at))]
async fn retrieve_batch(
    connection: &PgPool,
    starting_at: i32,
    min_offset: StdDuration,
) -> crate::Result<Batch> {
    // language=SQL
    const QUERY: &str = "
        SELECT account_id, last_battle_time FROM accounts
        WHERE account_id > $1 AND last_battle_time < now() - $2
        ORDER BY account_id LIMIT 100
    ";
    let query = sqlx::query_as(QUERY).bind(starting_at).bind(min_offset);
    timeout(StdDuration::from_secs(60), query.fetch_all(connection))
        .await
        .context("the `retrieve_batch` query has timed out")?
        .context("failed to retrieve a batch")
}
