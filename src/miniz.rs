pub fn decompress_to_vec(input: &[u8]) -> crate::Result<Vec<u8>> {
    miniz_oxide::inflate::decompress_to_vec(input)
        .map_err(|error| anyhow::anyhow!("failed to decompress the input: {:?}", error))
}
