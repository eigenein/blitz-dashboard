use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use futures::{stream, Stream};
use sqlx::PgPool;
use tokio::time::{sleep, timeout};

use crate::helpers::format_elapsed;
use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

/// Generates an infinite stream of batches, looping through the entire account table.
pub fn get_batch_stream(
    connection: PgPool,
    min_offset: StdDuration,
) -> impl Stream<Item = crate::Result<Batch>> {
    stream::try_unfold((0, Instant::now()), move |(mut pointer, mut start_time)| {
        let connection = connection.clone();
        async move {
            loop {
                let batch = retrieve_batch(&connection, pointer, min_offset).await?;
                match batch.last() {
                    Some(last_item) => {
                        let pointer = last_item.id;
                        break Ok(Some((batch, (pointer, start_time))));
                    }
                    None => {
                        tracing::info!(
                            elapsed = format_elapsed(&start_time).as_str(),
                            "restarting",
                        );
                        sleep(StdDuration::from_secs(1)).await;
                        start_time = Instant::now();
                        pointer = 0;
                    }
                }
            }
        }
    })
}

/// Retrieves a single account batch from the database.
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
