use chrono::Duration;

/// Specifies an account selection criteria for a batch stream.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Selector {
    /// Select accounts which last played sooner than the specified offset from now.
    /// Intended to scan accounts which are currently playing.
    /// The greater – the better, however, keep the maximum lag under 5-7 minutes.
    SoonerThan(Duration),

    /// Select accounts where last battle time is in between the specified offsets from now.
    /// Intended to scan accounts which have just started playing again after a pause,
    /// and allow «picking them up» by a «faster» sub-crawler.
    Between(Duration, Duration),

    /// Select accounts which last played earlier than the specified offset from now.
    /// Or, in other words, which haven't played for a long time.
    EarlierThan(Duration),

    /// Select all accounts.
    All,
}
