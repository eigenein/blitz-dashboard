pub use std::result::Result as StdResult;
pub use std::time::Duration as StdDuration;

pub use anyhow::anyhow;

#[allow(dead_code)]
pub type AHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

pub type DateTime = chrono::DateTime<chrono::Utc>;
pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;
