use std::ops::Add;
use std::time;

use poem::web::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};

use crate::math::statistics::ConfidenceLevel;
use crate::prelude::*;

/// Form & cookie.
#[derive(Deserialize, Default)]
pub struct UpdateDisplayPreferences {
    #[serde(default)]
    pub period: Option<Period>,

    #[serde(default)]
    pub confidence_level: Option<ConfidenceLevel>,
}

impl UpdateDisplayPreferences {
    pub const COOKIE_NAME: &'static str = "display-preferences";
}

impl From<Cookie> for UpdateDisplayPreferences {
    fn from(cookie: Cookie) -> Self {
        cookie
            .value::<UpdateDisplayPreferences>()
            .unwrap_or_default()
    }
}

impl From<&CookieJar> for UpdateDisplayPreferences {
    fn from(jar: &CookieJar) -> Self {
        jar.get(UpdateDisplayPreferences::COOKIE_NAME)
            .map(Self::from)
            .unwrap_or_default()
    }
}

impl Add<UpdateDisplayPreferences> for UpdateDisplayPreferences {
    type Output = UpdateDisplayPreferences;

    fn add(self, rhs: UpdateDisplayPreferences) -> Self::Output {
        Self {
            period: rhs.period.or(self.period),
            confidence_level: rhs.confidence_level.or(self.confidence_level),
        }
    }
}

/// Display preferences.
#[derive(Serialize, Hash)]
pub struct DisplayPreferences {
    pub period: Period,
    pub confidence_level: ConfidenceLevel,
}

impl From<UpdateDisplayPreferences> for DisplayPreferences {
    fn from(update: UpdateDisplayPreferences) -> Self {
        Self {
            period: update.period.unwrap_or_default(),
            confidence_level: update.confidence_level.unwrap_or_default(),
        }
    }
}

impl From<&CookieJar> for DisplayPreferences {
    fn from(jar: &CookieJar) -> Self {
        Self::from(UpdateDisplayPreferences::from(jar))
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct Period(pub StdDuration);

impl Default for Period {
    fn default() -> Self {
        Self(time::Duration::from_secs(86400))
    }
}

impl TryFrom<String> for Period {
    type Error = humantime::DurationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        humantime::parse_duration(&value).map(Self)
    }
}

impl From<Period> for String {
    fn from(period: Period) -> Self {
        humantime::format_duration(period.0).to_string()
    }
}
