use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::trainer::sample::Sample;

#[derive(Serialize, Deserialize)]
pub struct RecommendRequest {
    pub realm: wargaming::Realm,
    pub given: Vec<Given>,
    pub predict: Vec<wargaming::TankId>,
}

#[derive(Serialize, Deserialize)]
pub struct Given {
    pub tank_id: wargaming::TankId,
    pub sample: Sample,
}

impl From<&database::TankSnapshot> for Given {
    fn from(snapshot: &database::TankSnapshot) -> Self {
        Self {
            tank_id: snapshot.tank_id,
            sample: Sample {
                n_battles: snapshot.stats.n_battles,
                n_wins: snapshot.stats.n_wins,
            },
        }
    }
}
