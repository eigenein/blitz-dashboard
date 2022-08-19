use std::ops::Add;
use std::time;

use poem::web::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};

use crate::math::statistics::ConfidenceLevel;

/// Form & cookie.
#[serde_with::serde_as]
#[derive(Deserialize, Default)]
pub struct UpdateDisplayPreferences {
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub period: Option<time::Duration>,

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
#[serde_with::serde_as]
#[derive(Serialize, Hash)]
pub struct DisplayPreferences {
    #[serde_as(as = "serde_with::DurationSeconds")]
    pub period: time::Duration,

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
