use serde::Deserialize;
use validator::Validate;

use crate::wargaming;

pub const MIN_QUERY_LENGTH: usize = 3;
pub const MAX_QUERY_LENGTH: usize = 24;

#[derive(Deserialize, Validate)]
pub struct Params {
    #[serde(default)]
    #[validate(length(min = "MIN_QUERY_LENGTH", max = "MAX_QUERY_LENGTH"))]
    pub query: String,

    pub realm: wargaming::Realm,
}
