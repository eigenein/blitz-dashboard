use serde::Deserialize;

use crate::prelude::*;
use crate::wargaming;

pub const MIN_QUERY_LENGTH: usize = 3;
pub const MAX_QUERY_LENGTH: usize = 24;

#[derive(Deserialize)]
pub struct QueryParams {
    pub query: Query,
    pub realm: wargaming::Realm,
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub struct Query(pub String);

impl TryFrom<String> for Query {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() < MIN_QUERY_LENGTH {
            bail!("query is too short")
        }
        if value.len() > MAX_QUERY_LENGTH {
            bail!("query is tool long")
        }
        Ok(Self(value.to_lowercase()))
    }
}
