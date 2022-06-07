use std::str::FromStr;

use anyhow::anyhow;

use crate::prelude::*;

pub fn account_id(value: &str) -> Result<i32> {
    match i32::from_str(value)? {
        account_id if account_id >= 1 => Ok(account_id),
        account_id => Err(anyhow!("{} is an invalid account ID", account_id)),
    }
}

pub fn non_zero_usize(value: &str) -> Result<usize> {
    match FromStr::from_str(value)? {
        limit if limit >= 1 => Ok(limit),
        _ => Err(anyhow!("expected a positive number")),
    }
}

#[allow(dead_code)]
pub fn duration_as_secs<T>(value: &str) -> Result<T>
where
    T: TryFrom<u64>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    Ok(humantime::parse_duration(value)?.as_secs().try_into()?)
}
