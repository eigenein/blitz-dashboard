#[allow(dead_code)]
#[derive(Copy, Clone, Default)]
pub enum ConfidenceLevel {
    Z80,
    Z85,
    Z87,
    Z88,
    Z89,

    #[default]
    Z90,

    Z95,
    Z96,
    Z97,
    Z98,
    Z99,
    Z99_99,
}

impl ConfidenceLevel {
    pub const fn z_value(self) -> f64 {
        match self {
            Self::Z80 => 1.28,
            Self::Z85 => 1.440,
            Self::Z87 => 1.51,
            Self::Z88 => 1.5548,
            Self::Z89 => 1.598,
            Self::Z90 => 1.645,
            Self::Z95 => 1.960,
            Self::Z96 => 2.054,
            Self::Z97 => 2.17009,
            Self::Z98 => 2.326,
            Self::Z99 => 2.576,
            Self::Z99_99 => 3.29053,
        }
    }
}
