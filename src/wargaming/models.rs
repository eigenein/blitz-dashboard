use std::collections::HashMap;

pub use account_info::{AccountInfo, BaseAccountInfo};
pub use nation::Nation;
use serde::Deserialize;
pub use statistics::Statistics;
pub use tank_id::TankId;

pub mod account_info;
pub mod nation;
pub mod statistics;
pub mod tank_id;

pub type ResultMap<T> = HashMap<String, Option<T>>;
pub type ResultMapVec<T> = ResultMap<Vec<T>>;

/// Search accounts item.
#[derive(Deserialize, Debug, PartialEq)]
pub struct FoundAccount {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: i32,
}
