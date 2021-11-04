use std::fmt::{Display, Formatter};
use std::time::Duration as StdDuration;

use humantime::format_duration;

pub const fn from_minutes(minutes: u64) -> StdDuration {
    StdDuration::from_secs(minutes * 60)
}

pub const fn from_hours(hours: u64) -> StdDuration {
    from_minutes(hours * 60)
}

pub const fn from_days(days: u64) -> StdDuration {
    from_hours(days * 24)
}

pub const fn from_months(months: u64) -> StdDuration {
    StdDuration::from_secs(months * 2630016)
}

#[allow(dead_code)]
pub const fn from_years(years: u64) -> StdDuration {
    StdDuration::from_secs(years * 31557600)
}

pub struct Instant(std::time::Instant);

impl Instant {
    pub fn now() -> Self {
        Self(std::time::Instant::now())
    }

    pub fn elapsed(&self) -> Elapsed {
        Elapsed(self.0.elapsed())
    }
}

pub struct Elapsed(StdDuration);

impl Display for Elapsed {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&format_duration(self.0).to_string())
    }
}
