use crate::trainer::stream_entry::StreamEntry;
use crate::DateTime;

/// Single sample point of a dataset.
#[derive(Debug, Copy, Clone)]
pub struct SamplePoint {
    pub account_id: i32,
    pub tank_id: i32,
    pub timestamp: DateTime,
    pub is_win: bool,
    pub is_test: bool,
}

impl From<StreamEntry> for Vec<SamplePoint> {
    fn from(entry: StreamEntry) -> Self {
        (0..entry.n_battles)
            .map(|i| SamplePoint {
                account_id: entry.account_id,
                tank_id: entry.tank_id,
                timestamp: entry.timestamp,
                is_win: i < entry.n_wins,
                is_test: entry.is_test,
            })
            .collect()
    }
}
