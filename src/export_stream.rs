use tracing::info;

use crate::battle_stream::stream::BattleStream;
use crate::opts::ExportStreamOpts;

#[tracing::instrument(skip_all)]
pub async fn run(opts: ExportStreamOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "export-stream"));

    let redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let mut entries = BattleStream::read(redis, opts.time_span).await?.entries;

    if opts.sort_by_timestamp {
        entries.sort_by_key(|entry| entry.tank.timestamp);
    }

    for (i, entry) in entries.into_iter().enumerate() {
        if i % 100000 == 0 {
            info!(i);
        }
        println!("{}", serde_json::to_string(&entry)?);
    }

    Ok(())
}
