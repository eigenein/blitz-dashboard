use anyhow::anyhow;
use chrono::{TimeZone, Utc};

use crate::DateTime;

/// Single sample point of a dataset.
#[derive(Debug, Copy, Clone)]
pub struct StreamEntry {
    pub account_id: i32,
    pub tank_id: i32,
    pub timestamp: DateTime,
    pub n_battles: i32,
    pub n_wins: i32,
    pub is_test: bool,
}

pub struct StreamEntryBuilder {
    timestamp: Option<DateTime>,
    account_id: Option<i32>,
    tank_id: Option<i32>,
    n_battles: i32,
    n_wins: i32,
    is_test: bool,
}

impl Default for StreamEntryBuilder {
    fn default() -> Self {
        Self {
            timestamp: None,
            account_id: None,
            tank_id: None,
            n_battles: 1,
            n_wins: 0,
            is_test: false,
        }
    }
}

impl StreamEntryBuilder {
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

    pub fn n_wins(&mut self, n_wins: i32) -> &mut Self {
        self.n_wins = n_wins;
        self
    }

    pub fn n_battles(&mut self, n_battles: i32) -> &mut Self {
        self.n_battles = n_battles;
        self
    }

    pub fn set_test(&mut self, is_test: bool) -> &mut Self {
        self.is_test = is_test;
        self
    }

    pub fn build(&self) -> crate::Result<StreamEntry> {
        let point = StreamEntry {
            timestamp: self
                .timestamp
                .ok_or_else(|| anyhow!("timestamp is missing"))?,
            account_id: self
                .account_id
                .ok_or_else(|| anyhow!("account ID is missing"))?,
            tank_id: self.tank_id.ok_or_else(|| anyhow!("tank ID is missing"))?,
            is_test: self.is_test,
            n_battles: self.n_battles,
            n_wins: self.n_wins,
        };
        Ok(point)
    }
}
