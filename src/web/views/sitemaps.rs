use futures::StreamExt;
use poem::web::{Data, Path};
use poem::{handler, Body, IntoResponse, Response};

use crate::database;
use crate::prelude::*;

const CACHE_CONTROL: &str = "public, max-age=3600, stale-while-revalidate=86400";

#[handler]
#[instrument(skip_all, level = "info")]
pub async fn get_sitemap(
    db: Data<&mongodb::Database>,
    Path(realm): Path<String>,
) -> Result<impl IntoResponse> {
    let stream = database::AccountEntry::retrieve_page(&db, &realm, 50000)
        .await?
        .map(move |account| {
            account.map(|account| {
                format!("https://yastati.st/{}/{}\n", realm, account.id)
                    .as_bytes()
                    .to_vec()
            })
        });

    Ok(Response::from(Body::from_bytes_stream(stream))
        .with_header("Cache-Control", CACHE_CONTROL)
        .with_content_type("text/plain"))
}
