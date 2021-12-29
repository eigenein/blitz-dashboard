use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use futures::{stream, Stream};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{error, info, instrument};

use crate::database::retrieve_accounts;
use crate::helpers::format_elapsed;
use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

/// Account IDs from this list get crawled as soon as possible.
pub const PRIORITY_QUEUE_KEY: &str = "crawler::priority";

pub const PRIORITY_QUEUE_LIMIT: usize = 50;

const POINTER_KEY: &str = "crawler::pointer";

/// Comes from the Wargaming.net API limitation.
const MAX_BATCH_SIZE: usize = 100;

/// Generates an infinite stream of batches, looping through the entire account table.
pub async fn get_batch_stream(
    database: PgPool,
    mut redis: MultiplexedConnection,
    min_offset: Arc<RwLock<StdDuration>>,
) -> impl Stream<Item = crate::Result<Batch>> {
    let start_account_id = match redis.get::<_, Option<i32>>(POINTER_KEY).await {
        Ok(pointer) => pointer.unwrap_or(0),
        Err(error) => {
            error!("failed to retrieve the pointer: {:#}", error);
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
                    let priority_batch = retrieve_priority_queue(&database, &mut redis).await?;
                    let batch_size = MAX_BATCH_SIZE - priority_batch.len();
                    let min_offset = *min_offset.read().await;
                    let batch = retrieve_batch(&database, pointer, min_offset, batch_size).await?;
                    match batch.last() {
                        Some(last_item) => {
                            pointer = last_item.id;
                            redis.set(POINTER_KEY, pointer).await?;
                        }
                        None => {
                            info!(elapsed = %format_elapsed(&start_time), "restarting");
                            sleep(StdDuration::from_secs(1)).await;
                            start_time = Instant::now();
                            pointer = 0;
                        }
                    }
                    let batch: Batch = priority_batch
                        .into_iter()
                        .chain(batch.into_iter())
                        .collect();
                    if !batch.is_empty() {
                        break Ok(Some((batch, (pointer, start_time))));
                    }
                }
            }
        },
    )
}

/// Retrieves a single account batch from the database.
#[instrument(skip_all, level = "debug", fields(starting_at = starting_at))]
async fn retrieve_batch(
    database: &PgPool,
    starting_at: i32,
    min_offset: StdDuration,
    count: usize,
) -> crate::Result<Batch> {
    // language=SQL
    const QUERY: &str = "
        SELECT account_id, last_battle_time FROM accounts
        WHERE account_id > $1 AND last_battle_time < now() - $2
        ORDER BY account_id LIMIT $3
    ";
    let query = sqlx::query_as(QUERY)
        .bind(starting_at)
        .bind(min_offset)
        .bind(i32::try_from(count)?);
    timeout(StdDuration::from_secs(60), query.fetch_all(database))
        .await
        .context("the `retrieve_batch` query has timed out")?
        .context("failed to retrieve a batch")
}

#[instrument(level = "debug", skip_all)]
async fn retrieve_priority_queue(
    database: &PgPool,
    redis: &mut MultiplexedConnection,
) -> crate::Result<Batch> {
    let account_ids: Vec<i32> = redis::cmd("SPOP")
        .arg(PRIORITY_QUEUE_KEY)
        .arg(PRIORITY_QUEUE_LIMIT)
        .query_async(redis)
        .await?;
    let accounts = if !account_ids.is_empty() {
        retrieve_accounts(database, &account_ids).await?
    } else {
        Vec::new()
    };
    Ok(accounts)
}
