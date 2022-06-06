use async_compression::tokio::write::{ZstdDecoder, ZstdEncoder};
use async_compression::Level;
use tokio::io::AsyncWriteExt;

use crate::prelude::*;

#[instrument(level = "debug", skip_all, fields(n_bytes = input.len()))]
pub async fn compress(input: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZstdEncoder::with_quality(Vec::new(), Level::Fastest);
    encoder.write_all(input).await?;
    encoder.shutdown().await?;
    Ok(encoder.into_inner())
}

#[instrument(level = "debug", skip_all, fields(n_bytes = input.len()))]
pub async fn decompress(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZstdDecoder::new(Vec::new());
    decoder.write_all(input).await?;
    decoder.shutdown().await?;
    Ok(decoder.into_inner())
}
