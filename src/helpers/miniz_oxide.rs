use anyhow::anyhow;
use miniz_oxide::{deflate, inflate};
use tokio::task::spawn_blocking;
use tracing::instrument;

#[instrument(level = "debug", skip_all)]
pub async fn compress_to_vec(input: Vec<u8>, level: u8) -> crate::Result<Vec<u8>> {
    Ok(spawn_blocking(move || deflate::compress_to_vec(&input, level)).await?)
}

#[instrument(level = "debug", skip_all)]
pub async fn decompress_to_vec(input: Vec<u8>) -> crate::Result<Vec<u8>> {
    spawn_blocking(move || inflate::decompress_to_vec(&input))
        .await?
        .map_err(|error| anyhow!("failed to decompress the input: {:?}", error))
}
