use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

use crate::models::BaseAccountInfo;

pub struct Account {
    pub base: BaseAccountInfo,
    pub factors: Vec<f64>,
}

impl<'r> FromRow<'r, PgRow> for Account {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            base: FromRow::from_row(row)?,
            factors: row.try_get("factors")?,
        })
    }
}
