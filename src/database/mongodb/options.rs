use mongodb::options::{UpdateOptions, WriteConcern};

use crate::prelude::*;

#[inline]
pub fn upsert_options() -> UpdateOptions {
    let write_concern = WriteConcern::builder()
        .w_timeout(StdDuration::from_secs(5))
        .build();
    UpdateOptions::builder()
        .upsert(true)
        .write_concern(write_concern)
        .build()
}
