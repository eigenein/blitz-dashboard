use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Default, PartialEq, Debug)]
#[serde(from = "f64", into = "f64")]
pub struct MmRating(pub f64);

impl MmRating {
    #[must_use]
    pub fn display_rating(self) -> i32 {
        (self.0 * 10.0 + 3000.0) as i32
    }
}

impl From<f64> for MmRating {
    fn from(mm_rating: f64) -> Self {
        Self(mm_rating)
    }
}

impl From<MmRating> for f64 {
    fn from(mm_rating: MmRating) -> Self {
        mm_rating.0
    }
}
