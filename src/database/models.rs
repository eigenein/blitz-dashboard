use chrono::{TimeZone, Utc};
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

use crate::models::BaseAccountInfo;

pub struct Account {
    pub base: BaseAccountInfo,
    pub cf: CfProfile,
}

impl Account {
    pub fn empty(account_id: i32) -> Self {
        Self {
            base: BaseAccountInfo {
                id: account_id,
                last_battle_time: Utc.timestamp(0, 0),
            },
            cf: Default::default(),
        }
    }
}

impl<'r> FromRow<'r, PgRow> for Account {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            base: FromRow::from_row(row)?,
            cf: FromRow::from_row(row)?,
        })
    }
}

/// Collaborative filtering profile.
#[derive(Default)]
pub struct CfProfile {
    pub bias: f64,
    pub factors: Vec<f64>,
}

impl<'r> FromRow<'r, PgRow> for CfProfile {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            bias: row.try_get("bias")?,
            factors: row.try_get("factors")?,
        })
    }
}
