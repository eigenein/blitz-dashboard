use std::fmt::{Display, Formatter};

use humantime::format_duration;

use crate::StdDuration;

/// Specifies an account selection criteria for a batch stream.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Selector {
    /// Select accounts where last battle time is in between the specified offsets from now.
    /// Intended to scan accounts which have just started playing again after a pause,
    /// and allow «picking them up» by a «faster» sub-crawler.
    Between(StdDuration, StdDuration),

    /// Select accounts which have last played earlier than the specified offset from now.
    /// Or, in other words, which haven't played for a long time.
    Before(StdDuration),
}

impl Display for Selector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Between(offset_1, offset_2) => {
                write!(
                    f,
                    "BETWEEN [{}] and [{}] ago",
                    format_duration(*offset_2),
                    format_duration(*offset_1)
                )
            }
            Selector::Before(offset) => {
                write!(f, "BEFORE [{}] ago", format_duration(*offset))
            }
        }
    }
}
