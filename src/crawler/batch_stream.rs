use anyhow::Context;
use chrono::Duration;
use futures::{stream, Stream, StreamExt};
use sqlx::PgPool;

use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

#[derive(Debug, Copy, Clone)]
pub enum Select {
    /// Select accounts which played sooner than the specified offset.
    Hot,

    /// Select accounts which played earlier than the specified offset.
    Cold,
}

/// Generates an infinite stream of batches, looping through the entire account table.
pub fn loop_batches_from(
    connection: PgPool,
    select: Select,
    offset: Duration,
) -> impl Stream<Item = crate::Result<Batch>> {
    let initial_stream = Box::pin(get_batches_from(connection.clone(), select, offset));
    stream::unfold(
        (connection, initial_stream),
        move |(connection, mut inner_stream)| async move {
            match inner_stream.next().await {
                Some(item) => Some((item, (connection, inner_stream))),
                None => {
                    log::info!("{:?}: starting over.", select);
                    let mut new_stream =
                        Box::pin(get_batches_from(connection.clone(), select, offset));
                    new_stream
                        .next()
                        .await
                        .map(|item| (item, (connection, new_stream)))
                }
            }
        },
    )
}

/// Generates a finite stream of batches from the account table.
fn get_batches_from(
    connection: PgPool,
    select: Select,
    offset: Duration,
) -> impl Stream<Item = crate::Result<Batch>> {
    stream::try_unfold((connection, 0), move |(connection, pointer)| async move {
        let batch = retrieve_batch(&connection, pointer, select, offset).await?;
        match batch.last() {
            Some(item) => {
                let pointer = item.id;
                Ok(Some((batch, (connection, pointer))))
            }

            // FIXME: this doesn't necessarily mean that we've reached the table end.
            None => Ok(None),
        }
    })
}

async fn retrieve_batch(
    connection: &PgPool,
    starting_at: i32,
    select: Select,
    offset: Duration,
) -> crate::Result<Vec<BaseAccountInfo>> {
    // language=SQL
    let query: &str = match select {
        Select::Cold => {
            "SELECT * FROM accounts WHERE account_id > $1 AND last_battle_time < now() - $2 ORDER BY account_id LIMIT 100"
        }
        Select::Hot => {
            "SELECT * FROM accounts WHERE account_id > $1 AND last_battle_time > now() - $2 ORDER BY account_id LIMIT 100"
        },
    };
    let accounts = sqlx::query_as(query)
        .bind(starting_at)
        .bind(offset)
        .fetch_all(connection)
        .await
        .context("failed to retrieve a batch")?;
    Ok(accounts)
}