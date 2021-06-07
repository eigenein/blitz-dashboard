use std::collections::HashMap;

use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

/// Represents a player account.
/// Used to look up last updated timestamp.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    #[serde(rename = "aid")]
    pub id: i32,

    /// Timestamp when the document is updated in the database.
    #[serde(rename = "ts")]
    pub updated_at: DateTime,

    #[serde(rename = "lbts")]
    pub last_battle_time: DateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountUpdatedAt {
    #[serde(rename = "ts")]
    pub updated_at: DateTime,
}

/// Represents a snapshot of account statistics.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountSnapshot {
    #[serde(rename = "aid")]
    pub account_id: i32,

    #[serde(rename = "lbts")]
    pub last_battle_time: DateTime,

    #[serde(rename = "st")]
    pub statistics: StatisticsSnapshot,
}

/// Represents either a single account or a single player's tank statistics.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatisticsSnapshot {
    #[serde(rename = "nb")]
    pub battles: i32,

    #[serde(rename = "nw")]
    pub wins: i32,

    #[serde(rename = "ns")]
    pub survived_battles: i32,

    #[serde(rename = "nws")]
    pub win_and_survived: i32,

    #[serde(rename = "dmgd")]
    pub damage_dealt: i32,

    #[serde(rename = "dmgr")]
    pub damage_received: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TankSnapshot {
    #[serde(rename = "aid")]
    pub account_id: i32,

    #[serde(rename = "tid")]
    pub tank_id: i32,

    #[serde(rename = "blt")]
    pub battle_life_time: i64,

    #[serde(rename = "lbts")]
    pub last_battle_time: DateTime,

    #[serde(rename = "st")]
    pub statistics: StatisticsSnapshot,

    /// Keys here are hexadecimal CRC32's to save space in the database.
    #[serde(rename = "achv")]
    pub achievements: HashMap<String, i32>,

    /// Keys here are hexadecimal CRC32's to save space in the database.
    #[serde(rename = "mxs")]
    pub max_series: HashMap<String, i32>,
}
