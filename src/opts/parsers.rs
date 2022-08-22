use std::str::FromStr;

use anyhow::anyhow;
use clap::PossibleValue;

use crate::math::statistics::ConfidenceLevel;
use crate::prelude::*;
use crate::wargaming;

pub fn account_id(value: &str) -> Result<wargaming::AccountId> {
    match wargaming::AccountId::from_str(value)? {
        account_id if account_id >= 1 => Ok(account_id),
        account_id => Err(anyhow!("{} is an invalid account ID", account_id)),
    }
}

pub fn non_zero_usize(value: &str) -> Result<usize> {
    match FromStr::from_str(value)? {
        value if value >= 1 => Ok(value),
        _ => Err(anyhow!("expected a positive number")),
    }
}

pub fn non_zero_u32(value: &str) -> Result<u32> {
    match FromStr::from_str(value)? {
        value if value >= 1 => Ok(value),
        _ => Err(anyhow!("expected a positive number")),
    }
}

impl clap::ValueEnum for wargaming::Realm {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Russia, Self::Europe, Self::NorthAmerica, Self::Asia]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue<'a>> {
        match self {
            Self::Russia => Some(PossibleValue::new(Self::Russia.to_str())),
            Self::Europe => Some(PossibleValue::new(Self::Europe.to_str())),
            Self::Asia => Some(PossibleValue::new(Self::Asia.to_str())),
            Self::NorthAmerica => Some(PossibleValue::new(Self::NorthAmerica.to_str())),
        }
    }
}

impl clap::ValueEnum for ConfidenceLevel {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            ConfidenceLevel::Z50,
            ConfidenceLevel::Z70,
            ConfidenceLevel::Z75,
            ConfidenceLevel::Z80,
            ConfidenceLevel::Z85,
            ConfidenceLevel::Z87,
            ConfidenceLevel::Z88,
            ConfidenceLevel::Z89,
            ConfidenceLevel::Z90,
            ConfidenceLevel::Z95,
            ConfidenceLevel::Z96,
            ConfidenceLevel::Z97,
            ConfidenceLevel::Z98,
            ConfidenceLevel::Z99,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue<'a>> {
        match self {
            ConfidenceLevel::Z45 => Some(PossibleValue::new("45")),
            ConfidenceLevel::Z50 => Some(PossibleValue::new("50")),
            ConfidenceLevel::Z70 => Some(PossibleValue::new("70")),
            ConfidenceLevel::Z75 => Some(PossibleValue::new("75")),
            ConfidenceLevel::Z80 => Some(PossibleValue::new("80")),
            ConfidenceLevel::Z85 => Some(PossibleValue::new("85")),
            ConfidenceLevel::Z87 => Some(PossibleValue::new("87")),
            ConfidenceLevel::Z88 => Some(PossibleValue::new("88")),
            ConfidenceLevel::Z89 => Some(PossibleValue::new("89")),
            ConfidenceLevel::Z90 => Some(PossibleValue::new("90")),
            ConfidenceLevel::Z95 => Some(PossibleValue::new("95")),
            ConfidenceLevel::Z96 => Some(PossibleValue::new("96")),
            ConfidenceLevel::Z97 => Some(PossibleValue::new("97")),
            ConfidenceLevel::Z98 => Some(PossibleValue::new("98")),
            ConfidenceLevel::Z99 => Some(PossibleValue::new("99")),
        }
    }
}
