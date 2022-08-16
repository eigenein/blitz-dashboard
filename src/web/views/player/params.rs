use serde::Deserialize;

use crate::prelude::StdDuration;

#[derive(Deserialize)]
pub struct QueryParams {
    #[serde(default)]
    pub period: Option<Period>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Period(pub StdDuration);

impl TryFrom<String> for Period {
    type Error = humantime::DurationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        humantime::parse_duration(&value).map(Self)
    }
}
