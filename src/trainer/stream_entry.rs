use anyhow::anyhow;

use crate::wargaming::tank_id::TankId;

/// Single sample point of a dataset.
#[derive(Debug, Copy, Clone)]
pub struct StreamEntry {
    pub tank_id: TankId,
    pub timestamp: i64,
    pub n_battles: i32,
    pub n_wins: i32,
}

pub struct StreamEntryBuilder {
    timestamp: Option<i64>,
    tank_id: Option<TankId>,
    n_battles: i32,
    n_wins: i32,
}

impl Default for StreamEntryBuilder {
    fn default() -> Self {
        Self {
            timestamp: None,
            tank_id: None,
            n_battles: 1,
            n_wins: 0,
        }
    }
}

impl StreamEntryBuilder {
    pub fn timestamp(&mut self, secs: i64) -> &mut Self {
        self.timestamp = Some(secs);
        self
    }

    pub fn tank_id(&mut self, tank_id: TankId) -> &mut Self {
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

    pub fn build(&self) -> crate::Result<StreamEntry> {
        let point = StreamEntry {
            timestamp: self
                .timestamp
                .ok_or_else(|| anyhow!("timestamp is missing"))?,
            tank_id: self.tank_id.ok_or_else(|| anyhow!("tank ID is missing"))?,
            n_battles: self.n_battles,
            n_wins: self.n_wins,
        };
        Ok(point)
    }
}
