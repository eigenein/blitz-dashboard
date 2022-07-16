use serde::Deserialize;

pub use self::account_id::*;
pub use self::account_info::*;
pub use self::mm_rating::*;
pub use self::nation::*;
pub use self::realm::*;
pub use self::statistics::*;
pub use self::tank_achievements::*;
pub use self::tank_id::*;
pub use self::tank_stats::*;
pub use self::vehicle::*;

pub mod account_id;
pub mod account_info;
pub mod mm_rating;
pub mod nation;
pub mod realm;
pub mod statistics;
pub mod tank_achievements;
pub mod tank_id;
pub mod tank_stats;
pub mod vehicle;

/// Search accounts item.
#[derive(Deserialize, Debug, PartialEq)]
pub struct FoundAccount {
    pub nickname: String,

    #[serde(rename = "account_id")]
    pub id: AccountId,
}
