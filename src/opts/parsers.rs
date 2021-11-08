use std::str::FromStr;

use anyhow::anyhow;
use log::LevelFilter;

pub fn verbosity(n_occurences: u64) -> LevelFilter {
    match n_occurences {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

pub fn task_count(value: &str) -> crate::Result<usize> {
    match usize::from_str(value)? {
        count if count >= 1 => Ok(count),
        _ => Err(anyhow!("expected non-zero number of tasks")),
    }
}

pub fn account_id(value: &str) -> crate::Result<i32> {
    match i32::from_str(value)? {
        account_id if account_id >= 1 => Ok(account_id),
        account_id => Err(anyhow!("{} is an invalid account ID", account_id)),
    }
}

pub fn non_zero_usize(value: &str) -> crate::Result<usize> {
    match FromStr::from_str(value)? {
        limit if limit >= 1 => Ok(limit),
        _ => Err(anyhow!("expected a positive size")),
    }
}
