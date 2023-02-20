use std::ops::Add;
use std::time;

use poem::web::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};

/// Form & cookie.
#[serde_with::serde_as]
#[derive(Deserialize, Default)]
pub struct UpdateDisplayPreferences {
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub period: Option<time::Duration>,

    #[serde(default)]
    pub confidence_level_percentage: Option<f64>,

    pub target_victory_ratio_percentage: Option<f64>,
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
            confidence_level_percentage: rhs
                .confidence_level_percentage
                .or(self.confidence_level_percentage),
            target_victory_ratio_percentage: rhs
                .target_victory_ratio_percentage
                .or(self.target_victory_ratio_percentage),
        }
    }
}

/// Display preferences.
#[serde_with::serde_as]
#[derive(Serialize)]
pub struct DisplayPreferences {
    #[serde_as(as = "serde_with::DurationSeconds")]
    pub period: time::Duration,

    pub confidence_level_percentage: f64,

    pub confidence_level: f64,

    pub target_victory_ratio_percentage: f64,

    pub target_victory_ratio: f64,
}

impl From<UpdateDisplayPreferences> for DisplayPreferences {
    fn from(update: UpdateDisplayPreferences) -> Self {
        let confidence_level_percentage = update
            .confidence_level_percentage
            .map_or(90.0, |level| level.clamp(50.0, 99.99));
        let target_victory_ratio_percentage = update
            .target_victory_ratio_percentage
            .map_or(50.0, |level| level.clamp(0.01, 99.99));
        Self {
            period: update.period.unwrap_or(time::Duration::from_secs(86400)),
            confidence_level_percentage,
            confidence_level: confidence_level_percentage / 100.0,
            target_victory_ratio_percentage,
            target_victory_ratio: target_victory_ratio_percentage / 100.0,
        }
    }
}

impl From<&CookieJar> for DisplayPreferences {
    fn from(jar: &CookieJar) -> Self {
        Self::from(UpdateDisplayPreferences::from(jar))
    }
}
