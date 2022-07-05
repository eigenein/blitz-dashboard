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

impl Realm {
    /// Converts the realm to string.
    /// I would've just called `bson::to_bson`, but this is faster and infallible.
    #[inline]
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Asia => "asia",
            Self::Europe => "eu",
            Self::NorthAmerica => "na",
            Self::Russia => "ru",
        }
    }
}
