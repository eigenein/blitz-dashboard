use crate::battle_stream::stream::Stream;
use crate::opts::ExportStreamOpts;

#[tracing::instrument(skip_all)]
pub async fn run(opts: ExportStreamOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "export-stream"));

    let redis = ::redis::Client::open(opts.redis_uri.as_str())?
        .get_multiplexed_async_connection()
        .await?;
    let stream = Stream::read(redis, opts.time_span).await?;
    for entry in stream.entries {
        println!("{}", serde_json::to_string(&entry)?);
    }

    Ok(())
}
