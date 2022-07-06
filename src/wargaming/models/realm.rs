use anyhow::bail;
use rocket::FromFormField;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, FromFormField)]
pub enum Realm {
    #[field(value = "ru")]
    #[serde(rename = "ru")]
    Russia,

    #[field(value = "eu")]
    #[serde(rename = "eu")]
    Europe,

    #[field(value = "na")]
    #[serde(rename = "na")]
    NorthAmerica,

    #[field(value = "asia")]
    #[serde(rename = "asia")]
    Asia,
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
}

impl TryFrom<&str> for Realm {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "asia" => Ok(Self::Asia),
            "na" => Ok(Self::NorthAmerica),
            "eu" => Ok(Self::Europe),
            "ru" => Ok(Self::Russia),
            _ => bail!("`{}` is not a valid realm", value),
        }
    }
}
