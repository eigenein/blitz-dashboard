use futures::{stream, Stream, TryStreamExt};
use mongodb::Database;

use crate::database::mongodb::models::Account;
use crate::prelude::*;

pub fn get_account_stream(
    database: Database,
    sample_size: u32,
    min_offset: Duration,
    max_offset: Duration,
) -> impl Stream<Item = Result<Account>> {
    info!(sample_size, %min_offset, %max_offset);
    stream::try_unfold((1, database), move |(sample_number, database)| async move {
        debug!(sample_number, "retrieving a sample…");
        let samples =
            Account::retrieve_sample(&database, sample_size, min_offset, max_offset).await?;

        debug!(sample_number, "retrieved");
        Ok::<_, anyhow::Error>(Some((samples, (sample_number + 1, database))))
    })
    .try_flatten()
}
