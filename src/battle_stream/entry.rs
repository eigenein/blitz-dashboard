use anyhow::anyhow;
use serde::Serialize;

use crate::wargaming::tank_id::TankId;

/// Represents a single entry in the Redis stream
/// and contains many tanks of the same account.
pub struct StreamEntry {
    pub account_id: i32,
    pub tanks: Vec<TankEntry>,
}

impl StreamEntry {
    /// Converts the entry into an iterator of denormalized entries.
    pub fn into_denormalized(self) -> impl Iterator<Item = DenormalizedStreamEntry> {
        self.tanks
            .into_iter()
            .map(move |tank| DenormalizedStreamEntry {
                account_id: self.account_id,
                tank,
            })
    }
}

/// Represents a single tank from a stream entry.
#[derive(Serialize)]
pub struct TankEntry {
    pub tank_id: TankId,
    pub timestamp: i64,
    pub n_battles: i32,
    pub n_wins: i32,
}

/// Contains account ID in addition to a single account's tank.
/// Used to retain separate tank entries in the aggregator.
#[derive(Serialize)]
pub struct DenormalizedStreamEntry {
    pub account_id: i32,
    pub tank: TankEntry,
}

#[derive(Default)]
pub struct StreamEntryBuilder {
    account_id: Option<i32>,
    tanks: Vec<TankEntryBuilder>,
}

impl StreamEntryBuilder {
    pub fn account_id(&mut self, account_id: i32) -> &mut Self {
        self.account_id = Some(account_id);
        self
    }

    /// Starts a new tank entry with the specified tank ID.
    pub fn tank_id(&mut self, tank_id: TankId) -> &mut Self {
        self.tanks.push(TankEntryBuilder::new(tank_id));
        self
    }

    /// Sets the timestamp on the last tank.
    pub fn timestamp(&mut self, secs: i64) -> crate::Result<&mut Self> {
        let tank = self.tank()?;
        match tank.timestamp {
            None => {
                tank.timestamp = Some(secs);
                Ok(self)
            }
            Some(_) => Err(anyhow!("repeated timestamp for tank #{}", tank.tank_id)),
        }
    }

    /// Sets the number of wins on the last tank.
    pub fn n_wins(&mut self, n_wins: i32) -> crate::Result<&mut Self> {
        self.tank()?.n_wins = n_wins;
        Ok(self)
    }

    /// Sets the number of battles on the last tank.
    pub fn n_battles(&mut self, n_battles: i32) -> crate::Result<&mut Self> {
        self.tank()?.n_battles = n_battles;
        Ok(self)
    }

    pub fn build(&self) -> crate::Result<StreamEntry> {
        let entry = StreamEntry {
            account_id: self
                .account_id
                .ok_or_else(|| anyhow!("account ID is missing"))?,
            tanks: self
                .tanks
                .iter()
                .map(TankEntryBuilder::build)
                .collect::<crate::Result<Vec<TankEntry>>>()?,
        };
        Ok(entry)
    }

    /// Gets the current (last) tank from the list being constructed.
    fn tank(&mut self) -> crate::Result<&mut TankEntryBuilder> {
        self.tanks
            .last_mut()
            .ok_or_else(|| anyhow!("tank ID is expected first"))
    }
}

pub struct TankEntryBuilder {
    tank_id: TankId,
    timestamp: Option<i64>,
    n_battles: i32,
    n_wins: i32,
}

impl TankEntryBuilder {
    pub const DEFAULT_N_BATTLES: i32 = 1;
    pub const DEFAULT_N_WINS: i32 = 0;

    pub fn new(tank_id: TankId) -> Self {
        Self {
            tank_id,
            timestamp: None,
            n_battles: Self::DEFAULT_N_BATTLES,
            n_wins: Self::DEFAULT_N_WINS,
        }
    }

    pub fn build(&self) -> crate::Result<TankEntry> {
        let entry = TankEntry {
            tank_id: self.tank_id,
            timestamp: self
                .timestamp
                .ok_or_else(|| anyhow!("timestamp is missing"))?,
            n_battles: self.n_battles,
            n_wins: self.n_wins,
        };
        Ok(entry)
    }
}
