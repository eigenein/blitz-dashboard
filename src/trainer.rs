use crate::database;
use crate::opts::TrainOpts;
use crate::prelude::*;

pub async fn run(opts: TrainOpts) -> Result {
    let db = database::mongodb::open(&opts.connections.mongodb_uri).await?;
    let period = Duration::from_std(opts.train_period)?;
    Ok(())
}
