use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Copy, Clone, Default, Serialize, Deserialize, Hash, Debug)]
pub enum ConfidenceLevel {
    Z50,
    Z70,
    Z75,
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
}

impl ConfidenceLevel {
    pub const fn z_value(self) -> f64 {
        match self {
            Self::Z50 => 0.674490,
            Self::Z70 => 1.036433,
            Self::Z75 => 1.15035,
            Self::Z80 => 1.281551,
            Self::Z85 => 1.439531,
            Self::Z87 => 1.514101,
            Self::Z88 => 1.554773,
            Self::Z89 => 1.598193,
            Self::Z90 => 1.644853,
            Self::Z95 => 1.959964,
            Self::Z96 => 2.053749,
            Self::Z97 => 2.170091,
            Self::Z98 => 2.326348,
            Self::Z99 => 2.575829,
        }
    }
}
