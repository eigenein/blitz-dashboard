use anyhow::Context;
use futures::{stream, Stream, StreamExt};
use sqlx::PgPool;

use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

/// Generates an infinite stream of batches starting at `start_account_id`
/// and looping through the entire account table.
pub fn loop_batches(
    connection: PgPool,
    start_account_id: i32,
) -> impl Stream<Item = crate::Result<Batch>> + 'static {
    let initial_stream = Box::pin(get_batches(connection.clone(), start_account_id));
    stream::unfold(
        (connection, initial_stream),
        |(connection, mut inner_stream)| async move {
            match inner_stream.next().await {
                Some(item) => Some((item, (connection, inner_stream))),
                None => {
                    let mut new_stream = Box::pin(get_batches(connection.clone(), 0));
                    match new_stream.next().await {
                        Some(item) => Some((item, (connection, new_stream))),
                        None => None,
                    }
                }
            }
        },
    )
}

/// Generates a finite stream of batches starting at `account_id` till the end of the account table.
fn get_batches(
    connection: PgPool,
    account_id: i32,
) -> impl Stream<Item = crate::Result<Batch>> + 'static {
    stream::try_unfold(
        (connection, account_id),
        |(connection, pointer)| async move {
            // language=SQL
            const QUERY: &str = r#"
            SELECT * FROM accounts
            WHERE account_id > $1
            ORDER BY account_id 
            LIMIT 100
        "#;
            let batch: Batch = sqlx::query_as(QUERY)
                .bind(pointer)
                .fetch_all(&connection)
                .await
                .context("failed to retrieve a batch")?;
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
