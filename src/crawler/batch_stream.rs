use std::time::Duration as StdDuration;

use anyhow::Context;
use futures::{stream, Stream, StreamExt};
use sqlx::PgPool;

use crate::crawler::selector::Selector;
use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

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
                None => loop {
                    let mut new_stream = Box::pin(get_batches_stream(connection.clone(), selector));
                    if let Some(item) = new_stream.next().await {
                        break Some((item, (connection.clone(), new_stream)));
                    }
                    log::warn!("New stream is empty. Sleeping…");
                    tokio::time::sleep(StdDuration::from_secs(60)).await;
                },
            }
        },
    )
}

/// Generates a finite stream of batches from the account table.
fn get_batches_stream(
    connection: PgPool,
    selector: Selector,
) -> impl Stream<Item = crate::Result<Batch>> {
    log::info!("Starting stream: {:?}.", selector);
    stream::try_unfold((connection, 0), move |(connection, pointer)| async move {
        let batch = retrieve_batch(&connection, pointer, selector).await?;
        match batch.last() {
            Some(last_item) => {
                let pointer = last_item.id;
                Ok(Some((batch, (connection, pointer))))
            }
            None => Ok(None),
        }
    })
}

// TODO: move to `impl Selector`.
/// Retrieves a single account batch from the database.
async fn retrieve_batch(
    connection: &PgPool,
    starting_at: i32,
    selector: Selector,
) -> crate::Result<Vec<BaseAccountInfo>> {
    let query = match selector {
        Selector::All => {
            // language=SQL
            const QUERY: &str =
                "SELECT * FROM accounts WHERE account_id > $1 ORDER BY account_id LIMIT 100";
            sqlx::query_as(QUERY).bind(starting_at)
        }
        Selector::EarlierThan(min_offset) => {
            // language=SQL
            const QUERY: &str = "SELECT * FROM accounts WHERE account_id > $1 AND last_battle_time < now() - $2 ORDER BY account_id LIMIT 100";
            sqlx::query_as(QUERY).bind(starting_at).bind(min_offset)
        }
        Selector::SoonerThan(max_offset) => {
            // language=SQL
            const QUERY: &str = "SELECT * FROM accounts WHERE account_id > $1 AND last_battle_time > now() - $2 ORDER BY account_id LIMIT 100";
            sqlx::query_as(QUERY).bind(starting_at).bind(max_offset)
        }
        Selector::Between(min_offset, max_offset) => {
            assert!(min_offset < max_offset);
            // language=SQL
            const QUERY: &str = "
                SELECT * FROM accounts
                WHERE account_id > $1 AND last_battle_time BETWEEN SYMMETRIC now() - $2 AND now() - $3
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
