use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

use crate::models::BaseAccountInfo;
use crate::trainer::vector::Vector;

pub struct Account {
    pub base: BaseAccountInfo,
    pub factors: Vector,
}

impl<'r> FromRow<'r, PgRow> for Account {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            base: FromRow::from_row(row)?,
            factors: From::<Vec<f64>>::from(row.try_get("factors")?),
        })
    }
}
