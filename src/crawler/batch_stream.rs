use std::time::Duration as StdDuration;

use anyhow::Context;
use futures::{stream, Stream};
use sqlx::PgPool;
use tokio::time::{sleep, Instant};

use crate::crawler::selector::Selector;
use crate::models::BaseAccountInfo;

pub type Batch = Vec<BaseAccountInfo>;

/// Generates an infinite stream of batches, looping through the entire account table.
pub fn get_batch_stream(
    connection: PgPool,
    selector: Selector,
) -> impl Stream<Item = crate::Result<Batch>> {
    let mut start_instant = Instant::now();
    stream::try_unfold(0, move |mut pointer| {
        let connection = connection.clone();
        async move {
            loop {
                let batch = retrieve_batch(&connection, pointer, selector).await?;
                match batch.last() {
                    Some(last_item) => {
                        let pointer = last_item.id;
                        break Ok(Some((batch, pointer)));
                    }
                    None => {
                        let elapsed = start_instant.elapsed();
                        log::info!("Restarting {}: iteration took {:?}.", selector, elapsed);
                        sleep(StdDuration::from_secs(1)).await;
                        start_instant = Instant::now();
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
        Selector::LaterThan(max_offset) => {
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
