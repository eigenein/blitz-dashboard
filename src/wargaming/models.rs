pub use account_id::*;
pub use account_info::*;
pub use nation::*;
pub use realm::*;
use serde::Deserialize;
pub use statistics::*;
pub use tank_id::*;
pub use tank_statistics::*;
pub use vehicle::*;

pub mod account_id;
pub mod account_info;
pub mod nation;
pub mod realm;
pub mod statistics;
pub mod tank_id;
pub mod tank_statistics;
pub mod vehicle;

/// Search accounts item.
#[derive(Deserialize, Debug, PartialEq)]
pub struct FoundAccount {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: AccountId,
}
