use anyhow::Context;
use chrono::Duration;
use futures::{stream, Stream, StreamExt};
use sqlx::PgPool;

use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

#[derive(Debug, Copy, Clone)]
pub enum Selector {
    /// Select accounts which played sooner than the specified offset.
    Hot(Duration),

    /// Select accounts which played earlier than the specified offset.
    Frozen(Duration),

    /// Select accounts where last battle time is in between the specified offsets from now.
    Cold(Duration, Duration),
}

/// Generates an infinite stream of batches, looping through the entire account table.
pub fn get_infinite_batches_stream(
    connection: PgPool,
    selector: Selector,
) -> impl Stream<Item = crate::Result<Batch>> {
    let initial_stream = Box::pin(get_batches_stream(connection.clone(), selector));
    stream::unfold(
        (connection, initial_stream),
        move |(connection, mut inner_stream)| async move {
            match inner_stream.next().await {
                Some(item) => Some((item, (connection, inner_stream))),
                None => {
                    log::info!("{:?}: starting over.", selector);
                    let mut new_stream = Box::pin(get_batches_stream(connection.clone(), selector));
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
fn get_batches_stream(
    connection: PgPool,
    selector: Selector,
) -> impl Stream<Item = crate::Result<Batch>> {
    stream::try_unfold((connection, 0), move |(connection, pointer)| async move {
        let batch = retrieve_batch(&connection, pointer, selector).await?;
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
    selector: Selector,
) -> crate::Result<Vec<BaseAccountInfo>> {
    let query = match selector {
        Selector::Frozen(min_offset) => {
            // language=SQL
            const QUERY: &str = "SELECT * FROM accounts WHERE account_id > $1 AND last_battle_time < now() - $2 ORDER BY account_id LIMIT 100";
            sqlx::query_as(QUERY).bind(starting_at).bind(min_offset)
        }
        Selector::Hot(max_offset) => {
            // language=SQL
            const QUERY: &str = "SELECT * FROM accounts WHERE account_id > $1 AND last_battle_time > now() - $2 ORDER BY account_id LIMIT 100";
            sqlx::query_as(QUERY).bind(starting_at).bind(max_offset)
        }
        Selector::Cold(min_offset, max_offset) => {
            // language=SQL
            const QUERY: &str = "
                SELECT * FROM accounts
                WHERE account_id > $1 AND last_battle_time BETWEEN now() - $2 AND now() - $3
                ORDER BY account_id
                LIMIT 100
            ";
            sqlx::query_as(QUERY)
                .bind(starting_at)
                .bind(max_offset)
                .bind(min_offset)
        }
    };
    let accounts = query
        .fetch_all(connection)
        .await
        .context("failed to retrieve a batch")?;
    Ok(accounts)
}
