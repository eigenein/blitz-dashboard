use std::str::FromStr;

use anyhow::anyhow;
use clap::builder::PossibleValue;

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

#[allow(dead_code)]
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

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Russia => Some(PossibleValue::new(Self::Russia.to_str())),
            Self::Europe => Some(PossibleValue::new(Self::Europe.to_str())),
            Self::Asia => Some(PossibleValue::new(Self::Asia.to_str())),
            Self::NorthAmerica => Some(PossibleValue::new(Self::NorthAmerica.to_str())),
        }
    }
}
