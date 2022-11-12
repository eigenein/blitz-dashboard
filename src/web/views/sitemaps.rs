use futures::StreamExt;
use poem::web::{Data, Path};
use poem::{handler, Body, IntoResponse, Response};

use crate::database;
use crate::prelude::*;

const CACHE_CONTROL: &str = "no-cache";

#[handler]
#[instrument(skip_all, level = "info")]
pub async fn get_sitemap(
    db: Data<&mongodb::Database>,
    Path(realm): Path<String>,
) -> Result<impl IntoResponse> {
    let start_instant = Instant::now();
    let stream = database::AccountIdProjection::retrieve_recently_active(&db, &realm, 1000).await?;
    info!(elapsed_secs = ?start_instant.elapsed(), "stream ready");
    let stream = stream.map(move |account| {
        account.map(|account| format!("https://yastati.st/{}/{}\n", realm, account.id).into_bytes())
    });

    Ok(Response::from(Body::from_bytes_stream(stream))
        .with_header("Cache-Control", CACHE_CONTROL)
        .with_content_type("text/plain"))
}
