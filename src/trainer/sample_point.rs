use crate::trainer::stream_entry::StreamEntry;
use crate::wargaming::tank_id::TankId;

/// Single sample point of a dataset.
#[derive(Debug, Copy, Clone)]
pub struct SamplePoint {
    pub timestamp: i64,
    pub account_id: i32,
    pub tank_id: TankId,
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

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn size_ok() {
        assert_eq!(size_of::<SamplePoint>(), 16);
    }
}
