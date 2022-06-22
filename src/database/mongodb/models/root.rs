use serde::Deserialize;

/// Wrapper for aggregated results.
#[derive(Deserialize)]
pub struct Root<T> {
    #[serde(rename = "root")]
    pub root: T,
}
