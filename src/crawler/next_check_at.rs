use rand::prelude::*;
use rand_distr::Exp1;

use crate::prelude::*;

pub struct NextCheckAt(DateTime);

impl NextCheckAt {
    const MAX_EXP1_SAMPLE: f64 = 4.0;
}

impl From<DateTime> for NextCheckAt {
    fn from(last_battle_time: DateTime) -> Self {
        let elapsed_secs = (now() - last_battle_time).num_seconds() as f64;
        let sample = thread_rng()
            .sample::<f64, _>(Exp1)
            .min(Self::MAX_EXP1_SAMPLE);
        let interval_secs = elapsed_secs * sample;
        Self(last_battle_time + Duration::seconds(interval_secs as i64))
    }
}

impl From<NextCheckAt> for DateTime {
    fn from(next_check_at: NextCheckAt) -> Self {
        next_check_at.0
    }
}
