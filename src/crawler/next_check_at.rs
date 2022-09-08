use rand::prelude::*;
use rand_distr::Exp1;

use crate::prelude::*;

pub struct NextCheckAt {
    last_battle_time: DateTime,
    offset: f64,
    scale: f64,
    max_exp1: f64,
}

impl NextCheckAt {
    pub const fn new(last_battle_time: DateTime) -> Self {
        Self {
            last_battle_time,
            offset: 0.0,
            scale: 1.0,
            max_exp1: 3.5,
        }
    }
}

impl From<NextCheckAt> for DateTime {
    fn from(this: NextCheckAt) -> Self {
        let elapsed_secs = (now() - this.last_battle_time).num_seconds() as f64;
        let sample = thread_rng().sample::<f64, _>(Exp1) * this.scale + this.offset;
        let sample = sample.min(this.max_exp1);
        let next_check_in = Duration::seconds((elapsed_secs * sample) as i64);
        this.last_battle_time + next_check_in
    }
}
