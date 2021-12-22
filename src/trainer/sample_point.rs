use anyhow::anyhow;
use chrono::{TimeZone, Utc};
use enumflags2::{bitflags, BitFlags};

use crate::DateTime;

/// Single sample point of a dataset.
#[derive(Debug, Copy, Clone)]
pub struct SamplePoint {
    pub account_id: i32,
    pub tank_id: i32,
    pub flags: BitFlags<SamplePointFlags>,
    pub timestamp: DateTime,
}

impl SamplePoint {
    #[must_use]
    pub fn is_win(&self) -> bool {
        self.flags.contains(SamplePointFlags::Win)
    }

    #[must_use]
    pub fn is_test(&self) -> bool {
        self.flags.contains(SamplePointFlags::Test)
    }
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SamplePointFlags {
    /// The sample point represents a won battle, and a lost one otherwise.
    Win = 0b_0000_0001,

    /// The sample point belongs to the test set, and to the train set otherwise.
    Test = 0b_0000_0010,
}

#[derive(Default)]
pub struct SamplePointBuilder {
    timestamp: Option<DateTime>,
    account_id: Option<i32>,
    tank_id: Option<i32>,
    flags: BitFlags<SamplePointFlags>,
}

impl SamplePointBuilder {
    pub fn timestamp(&mut self, timestamp: DateTime) -> &mut Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn timestamp_secs(&mut self, secs: i64) -> &mut Self {
        self.timestamp(Utc.timestamp(secs, 0))
    }

    pub fn account_id(&mut self, account_id: i32) -> &mut Self {
        self.account_id = Some(account_id);
        self
    }

    pub fn tank_id(&mut self, tank_id: i32) -> &mut Self {
        self.tank_id = Some(tank_id);
        self
    }

    pub fn win(&mut self) -> &mut Self {
        self.flags.insert(SamplePointFlags::Win);
        self
    }

    pub fn test(&mut self) -> &mut Self {
        self.flags.insert(SamplePointFlags::Test);
        self
    }

    pub fn build(&self) -> crate::Result<SamplePoint> {
        let point = SamplePoint {
            timestamp: self
                .timestamp
                .ok_or_else(|| anyhow!("timestamp is missing"))?,
            account_id: self
                .account_id
                .ok_or_else(|| anyhow!("account ID is missing"))?,
            tank_id: self.tank_id.ok_or_else(|| anyhow!("tank ID is missing"))?,
            flags: self.flags,
        };
        Ok(point)
    }
}
