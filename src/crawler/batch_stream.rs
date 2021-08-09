use anyhow::Context;
use futures::{stream, Stream, StreamExt};
use sqlx::PgPool;

use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

/// Generates an infinite stream of batches starting at `starting_account_id`
/// and looping through the entire account table.
pub fn loop_batches(
    connection: PgPool,
    starting_account_id: i32,
) -> impl Stream<Item = crate::Result<Batch>> {
    // Starting at `starting_account_id`.
    let initial_stream = Box::pin(get_batches(connection.clone(), starting_account_id));
    stream::unfold(
        (connection, initial_stream),
        |(connection, mut inner_stream)| async move {
            match inner_stream.next().await {
                // Exhaust the current stream.
                Some(item) => Some((item, (connection, inner_stream))),

                // The current stream has ended, starting over and return the new stream.
                None => {
                    log::info!("Starting over.");
                    let mut new_stream = Box::pin(get_batches(connection.clone(), 0));
                    new_stream
                        .next()
                        .await
                        .map(|item| (item, (connection, new_stream)))
                }
            }
        },
    )
}

/// Generates a finite stream of batches starting at `account_id` till the end of the account table.
fn get_batches(connection: PgPool, account_id: i32) -> impl Stream<Item = crate::Result<Batch>> {
    stream::try_unfold(
        (connection, account_id),
        |(connection, pointer)| async move {
            let batch = retrieve_batch(&connection, pointer).await?;
            match batch.last() {
                Some(item) => {
                    let pointer = item.id;
                    Ok(Some((batch, (connection, pointer))))
                }
                None => Ok(None),
            }
        },
    )
}

async fn retrieve_batch(
    connection: &PgPool,
    starting_at: i32,
) -> crate::Result<Vec<BaseAccountInfo>> {
    // language=SQL
    const QUERY: &str = r#"
            SELECT * FROM accounts
            WHERE account_id > $1
            ORDER BY account_id 
            LIMIT 100
        "#;
    let accounts = sqlx::query_as(QUERY)
        .bind(starting_at)
        .fetch_all(connection)
        .await
        .context("failed to retrieve a batch")?;
    Ok(accounts)
}
