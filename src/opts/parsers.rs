use std::str::FromStr;

use anyhow::anyhow;

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
