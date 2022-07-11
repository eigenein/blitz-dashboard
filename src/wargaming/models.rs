use std::collections::HashMap;

pub use account_id::*;
pub use account_info::*;
use itertools::{merge_join_by, EitherOrBoth};
pub use nation::*;
pub use realm::*;
use serde::Deserialize;
pub use statistics::*;
pub use tank::*;
pub use tank_id::*;
pub use tank_statistics::*;
pub use vehicle::*;

use crate::prelude::*;

pub mod account_id;
pub mod account_info;
pub mod nation;
pub mod realm;
pub mod statistics;
pub mod tank;
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
    pub id: AccountId,
}

/// Merges tank statistics and tank achievements into a single tank structure.
pub fn merge_tanks(
    account_id: AccountId,
    mut statistics: Vec<TankStats>,
    mut achievements: Vec<TankAchievements>,
) -> AHashMap<TankId, Tank> {
    statistics.sort_unstable_by_key(|snapshot| snapshot.tank_id);
    achievements.sort_unstable_by_key(|achievements| achievements.tank_id);

    merge_join_by(statistics, achievements, |left, right| left.tank_id.cmp(&right.tank_id))
        .filter_map(|item| match item {
            EitherOrBoth::Both(statistics, achievements) => Some((
                statistics.tank_id,
                Tank {
                    account_id,
                    statistics,
                    achievements,
                },
            )),
            _ => None,
        })
        .collect()
}
