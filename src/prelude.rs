pub use std::sync::Arc;
pub use std::time;
pub use std::time::Instant;

pub use anyhow::{anyhow, bail, Context, Error};
pub use async_trait::async_trait;
pub use chrono::{Datelike, Duration, TimeZone, Utc};
pub use serde_with::TryFromInto;
pub use tracing::{debug, debug_span, error, info, info_span, instrument, trace, warn};
pub use tracing_futures::Instrument;

pub use crate::{database, wargaming};

#[allow(dead_code)]
pub type AHashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

#[allow(dead_code)]
pub type AHashSet<V> = std::collections::HashSet<V, ahash::RandomState>;

pub type DateTime = chrono::DateTime<Utc>;
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[inline]
pub fn now() -> DateTime {
    Utc::now()
}
