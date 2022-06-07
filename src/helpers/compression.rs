use async_compression::tokio::write::{ZstdDecoder, ZstdEncoder};
use async_compression::Level;
use tokio::io::AsyncWriteExt;

use crate::prelude::*;

#[instrument(level = "debug", skip_all)]
pub async fn compress(input: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZstdEncoder::with_quality(Vec::new(), Level::Fastest);
    encoder.write_all(input).await?;
    encoder.shutdown().await?;
    let output = encoder.into_inner();
    debug!(input_len = input.len(), output_len = output.len(), "compressed");
    Ok(output)
}

#[instrument(level = "debug", skip_all)]
pub async fn decompress(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZstdDecoder::new(Vec::new());
    decoder.write_all(input).await?;
    decoder.shutdown().await?;
    let output = decoder.into_inner();
    debug!(input_len = input.len(), output_len = output.len(), "decompressed");
    Ok(output)
}
