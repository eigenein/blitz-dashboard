use bytes::Bytes;
use tokio::task::spawn_blocking;

pub async fn compress_to_vec(input: Vec<u8>, level: u8) -> crate::Result<Vec<u8>> {
    Ok(spawn_blocking(move || miniz_oxide::deflate::compress_to_vec(&input, level)).await?)
}

pub async fn decompress_to_vec(input: Bytes) -> crate::Result<Vec<u8>> {
    spawn_blocking(move || miniz_oxide::inflate::decompress_to_vec(&input))
        .await?
        .map_err(|error| anyhow::anyhow!("failed to decompress the input: {:?}", error))
}
