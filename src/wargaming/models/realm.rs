use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum Realm {
    #[serde(rename = "ru")]
    Russia,

    #[serde(rename = "eu")]
    Europe,

    #[serde(rename = "na")]
    NorthAmerica,

    #[serde(rename = "asia")]
    Asia,
}

impl fmt::Display for Realm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Realm {
    /// Converts the realm to string.
    /// I would've just called `bson::to_bson`, but this is faster and infallible.
    #[inline]
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Asia => "asia",
            Self::Europe => "eu",
            Self::NorthAmerica => "na",
            Self::Russia => "ru",
        }
    }

    #[inline]
    pub const fn to_emoji(self) -> &'static str {
        match self {
            Self::Asia => "π¨π³",
            Self::Europe => "πͺπΊ",
            Self::NorthAmerica => "πΊπΈ",
            Self::Russia => "π·πΊ",
        }
    }
}
