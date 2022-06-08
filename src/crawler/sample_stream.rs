use futures::{stream, Stream, TryStreamExt};
use mongodb::Database;

use crate::database::mongodb::models::Account;
use crate::prelude::*;

pub fn get_sample_stream(
    database: Database,
    sample_size: u32,
    min_offset: Duration,
    max_offset: Duration,
) -> impl Stream<Item = Result<Vec<Account>>> {
    stream::try_unfold(database, move |database| async move {
        let samples =
            Account::retrieve_samples(&database, sample_size, min_offset, max_offset).await?;
        Ok::<_, anyhow::Error>(Some((samples, database)))
    })
    .try_flatten()
}
