pub use std::result::Result as StdResult;
pub use std::time::Duration as StdDuration;

pub use anyhow::anyhow;
pub use chrono::{Datelike, Duration, TimeZone, Utc};
pub use tracing::{debug, error, info, instrument, trace, warn};

#[allow(dead_code)]
pub type AHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

pub type DateTime = chrono::DateTime<Utc>;
pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;
