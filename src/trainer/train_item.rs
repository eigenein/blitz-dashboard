use crate::prelude::*;

pub struct CompressedTrainItem {
    pub account_id: wargaming::AccountId,
    pub tank_id: wargaming::TankId,
    pub last_battle_time: i64,
    pub n_battles: u16,
    pub n_wins: u16,
}

impl TryFrom<database::TrainItem> for CompressedTrainItem {
    type Error = Error;

    fn try_from(item: database::TrainItem) -> Result<Self, Self::Error> {
        Ok(Self {
            account_id: item.account_id,
            tank_id: item.tank_id,
            last_battle_time: item.last_battle_time.timestamp(),
            n_battles: item.n_battles.try_into()?,
            n_wins: item.n_wins.try_into()?,
        })
    }
}
