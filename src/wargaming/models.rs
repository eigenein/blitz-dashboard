use std::collections::HashMap;

pub use account_info::*;
pub use nation::*;
use serde::Deserialize;
pub use statistics::*;
pub use tank_id::*;
pub use tank_statistics::*;
pub use vehicle::*;

pub mod account_info;
pub mod nation;
pub mod statistics;
pub mod tank_id;
pub mod tank_statistics;
pub mod vehicle;

#[allow(dead_code)]
pub type ResultMap<T> = HashMap<String, Option<T>>;

#[allow(dead_code)]
pub type ResultMapVec<T> = ResultMap<Vec<T>>;

/// Search accounts item.
#[derive(Deserialize, Debug, PartialEq)]
pub struct FoundAccount {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: i32,
}
