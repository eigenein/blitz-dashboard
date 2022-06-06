pub use nation::Nation;
use serde::Deserialize;
pub use tank_id::TankId;

pub mod nation;
pub mod tank_id;

/// Search accounts item.
#[derive(Deserialize, Debug, PartialEq)]
pub struct FoundAccount {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: i32,
}
