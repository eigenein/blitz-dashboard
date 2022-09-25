use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Copy, Clone, Default, Serialize, Deserialize, Hash, Debug)]
pub enum ConfidenceLevel {
    Z45,
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
