use crate::math::statistics::ConfidenceInterval;

pub trait NWins {
    fn n_wins(&self) -> i32;
}

pub trait NBattles {
    fn n_battles(&self) -> i32;
}

pub trait DamageDealt {
    fn damage_dealt(&self) -> i32;
}

pub trait TrueWinRate {
    fn true_win_rate(&self) -> ConfidenceInterval;
}

pub trait MMRating {
    fn mm_rating(&self) -> f64;

    #[must_use]
    fn display_rating(&self) -> i32 {
        (self.mm_rating() * 10.0 + 3000.0).round() as i32
    }
}

impl<T: NBattles + NWins> TrueWinRate for T {
    fn true_win_rate(&self) -> ConfidenceInterval {
        ConfidenceInterval::wilson_score_interval(
            self.n_battles(),
            self.n_wins(),
            Default::default(),
        )
    }
}

pub trait CurrentWinRate {
    fn current_win_rate(&self) -> f64;
}

impl<T: NBattles + NWins> CurrentWinRate for T {
    fn current_win_rate(&self) -> f64 {
        self.n_wins() as f64 / self.n_battles() as f64
    }
}

pub trait AverageDamageDealt {
    fn average_damage_dealt(&self) -> f64;
}

impl<T: NBattles + DamageDealt> AverageDamageDealt for T {
    fn average_damage_dealt(&self) -> f64 {
        self.damage_dealt() as f64 / self.n_battles() as f64
    }
}
