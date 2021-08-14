//! Helper functions for [`std::time`].

use std::time::Duration as StdDuration;

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
