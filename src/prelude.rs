pub use std::result::Result as StdResult;
pub use std::time::{Duration as StdDuration, Instant};

pub use anyhow::{anyhow, Context};
pub use chrono::{Datelike, Duration, TimeZone, Utc};
pub use tracing::{debug, debug_span, error, info, info_span, instrument, trace, warn, Instrument};

#[allow(dead_code)]
pub type AHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

pub type DateTime = chrono::DateTime<Utc>;
pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;
