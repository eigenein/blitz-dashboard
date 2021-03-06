use serde::Deserialize;

use crate::prelude::StdDuration;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub period: Period,
}

#[derive(Copy, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct Period(pub StdDuration);

impl Default for Period {
    fn default() -> Self {
        Self(StdDuration::from_secs(86400))
    }
}

impl TryFrom<String> for Period {
    type Error = humantime::DurationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        humantime::parse_duration(&value).map(Self)
    }
}
