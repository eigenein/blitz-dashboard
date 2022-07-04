use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Default)]
pub enum Realm {
    #[default] // TODO: eventually drop.
    #[serde(rename = "ru")]
    Russia,

    #[serde(rename = "eu")]
    Europe,

    #[serde(rename = "na")]
    NorthAmerica,

    #[serde(rename = "asia")]
    Asia,
}
