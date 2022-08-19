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

    pub target_victory_ratio: Option<TargetVictoryRatio>,
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
            target_victory_ratio: rhs.target_victory_ratio.or(self.target_victory_ratio),
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

    pub target_victory_ratio: TargetVictoryRatio,
}

impl From<UpdateDisplayPreferences> for DisplayPreferences {
    fn from(update: UpdateDisplayPreferences) -> Self {
        Self {
            period: update.period.unwrap_or(time::Duration::from_secs(86400)),
            confidence_level: update.confidence_level.unwrap_or_default(),
            target_victory_ratio: update.target_victory_ratio.unwrap_or_default(),
        }
    }
}

impl From<&CookieJar> for DisplayPreferences {
    fn from(jar: &CookieJar) -> Self {
        Self::from(UpdateDisplayPreferences::from(jar))
    }
}

/// Target victory ratio defines which value we'll use to provide
/// the «semaphore» hints to the user.
#[must_use]
#[derive(Serialize, Deserialize, Default, Copy, Clone, Hash)]
pub enum TargetVictoryRatio {
    #[default]
    Current,

    P50,
    P55,
    P60,
    P65,
    P70,
    P75,
    P80,
    P85,
    P90,
    P95,
}

impl TargetVictoryRatio {
    pub fn custom_or_else<F: FnOnce() -> f64>(self, current: F) -> f64 {
        match self {
            Self::P50 => 0.50,
            Self::P55 => 0.55,
            Self::P60 => 0.60,
            Self::P65 => 0.65,
            Self::P70 => 0.70,
            Self::P75 => 0.75,
            Self::P80 => 0.80,
            Self::P85 => 0.85,
            Self::P90 => 0.90,
            Self::P95 => 0.95,
            Self::Current => current(),
        }
    }
}
