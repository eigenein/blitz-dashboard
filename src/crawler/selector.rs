use std::fmt::{Display, Formatter};

use chrono::Duration;

/// Specifies an account selection criteria for a batch stream.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Selector {
    /// Select accounts which have last played later than the specified offset from now.
    /// Intended to scan accounts which are currently playing.
    /// The greater – the better, however, keep the maximum lag under 5-7 minutes.
    LaterThan(Duration),

    /// Select accounts where last battle time is in between the specified offsets from now.
    /// Intended to scan accounts which have just started playing again after a pause,
    /// and allow «picking them up» by a «faster» sub-crawler.
    Between(Duration, Duration),

    /// Select accounts which have last played earlier than the specified offset from now.
    /// Or, in other words, which haven't played for a long time.
    EarlierThan(Duration),

    /// Select all accounts.
    All,
}

impl Display for Selector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::LaterThan(offset) => write!(f, "LATER than {} ago", offset),
            Selector::Between(offset_1, offset_2) => {
                write!(f, "BETWEEN {} and {} ago", offset_2, offset_1)
            }
            Selector::EarlierThan(offset) => write!(f, "EARLIER than {} ago", offset),
            Selector::All => write!(f, "ALL accounts"),
        }
    }
}
