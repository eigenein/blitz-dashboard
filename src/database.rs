use std::time::{Duration, Instant};

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use rusqlite::{OptionalExtension, ToSql};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::models::{AccountInfo, BasicAccountInfo, TankSnapshot};

pub struct Database(rusqlite::Connection);

impl Database {
    /// Open and initialize the database.
    pub fn open<P: Into<String>>(path: P) -> crate::Result<Self> {
        let path = path.into();

        log::info!("Connecting to the database…");
        let inner = rusqlite::Connection::open(&path)?;
        inner.busy_timeout(Duration::from_secs(5))?;

        log::info!("Initializing the database schema…");
        // language=SQL
        inner.execute_batch(
            r#"
            -- noinspection SqlSignatureForFile
            -- noinspection SqlResolveForFile @ routine/"json_extract"
            
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = normal;

            CREATE TABLE IF NOT EXISTS accounts (document JSON NOT NULL);
            CREATE UNIQUE INDEX IF NOT EXISTS accounts_account_id
                ON accounts(json_extract(document, '$.account_id') ASC);
            CREATE INDEX IF NOT EXISTS accounts_crawled_at
                ON accounts(json_extract(document, '$.crawled_at') ASC);

            CREATE TABLE IF NOT EXISTS account_snapshots (document JSON NOT NULL);
            CREATE UNIQUE INDEX IF NOT EXISTS account_snapshots_account_id_last_battle_time
                ON account_snapshots(
                    json_extract(document, '$.last_battle_time') DESC,
                    json_extract(document, '$.account_id') ASC
                );

            CREATE TABLE IF NOT EXISTS tank_snapshots (document JSON NOT NULL);
            CREATE UNIQUE INDEX IF NOT EXISTS tank_snapshots_account_id_tank_id_last_battle_time
                ON tank_snapshots(
                    json_extract(document, '$.last_battle_time') DESC,
                    json_extract(document, '$.account_id') ASC,
                    json_extract(document, '$.tank_id') ASC
                );
            "#,
        )?;

        Ok(Self(inner))
    }

    pub fn transaction(&self) -> crate::Result<Transaction> {
        Ok(Transaction(self.0.unchecked_transaction()?))
    }

    pub fn get_account_count(&self) -> crate::Result<i64> {
        // language=SQL
        Ok(self
            .0
            .prepare_cached("SELECT count(*) FROM accounts")?
            .query_row([], |row| row.get(0))?)
    }

    pub fn get_account_snapshot_count(&self) -> crate::Result<i64> {
        // language=SQL
        Ok(self
            .0
            .prepare_cached("SELECT count(*) FROM account_snapshots")?
            .query_row([], |row| row.get(0))?)
    }

    pub fn get_tank_snapshot_count(&self) -> crate::Result<i64> {
        // language=SQL
        Ok(self
            .0
            .prepare_cached("SELECT count(*) FROM tank_snapshots")?
            .query_row([], |row| row.get(0))?)
    }

    pub fn get_oldest_account(&self) -> crate::Result<Option<BasicAccountInfo>> {
        // language=SQL
        Ok(self
            .0
            .prepare_cached("SELECT document FROM accounts ORDER BY json_extract(document, '$.crawled_at') LIMIT 1")?
            .query_row([], |row| row.get(0))
            .optional()?
        )
    }

    pub fn upsert_account(&self, info: &BasicAccountInfo) -> crate::Result {
        // language=SQL
        self.0
            .prepare_cached("INSERT OR REPLACE INTO accounts (document) VALUES (?1)")?
            .execute([info])?;
        Ok(())
    }

    pub fn upsert_account_snapshot(&self, info: &AccountInfo) -> crate::Result {
        // language=SQL
        self.0
            .prepare_cached("INSERT OR REPLACE INTO account_snapshots (document) VALUES (?1)")?
            .execute([info])?;
        Ok(())
    }

    pub fn upsert_tanks(&self, tanks: &[TankSnapshot]) -> crate::Result {
        let start_instant = Instant::now();
        // language=SQL
        let mut statement = self
            .0
            .prepare_cached("INSERT OR IGNORE INTO tank_snapshots (document) VALUES (?1)")?;
        for tank in tanks {
            statement.execute(&[tank])?;
        }
        log::debug!(
            "{} tanks upserted in {:?}.",
            tanks.len(),
            Instant::now() - start_instant,
        );
        Ok(())
    }
}

pub struct Transaction<'c>(rusqlite::Transaction<'c>);

impl Transaction<'_> {
    pub fn commit(self) -> crate::Result {
        Ok(self.0.commit()?)
    }
}

impl ToSql for BasicAccountInfo {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        serializable_to_sql(self)
    }
}

impl ToSql for AccountInfo {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        serializable_to_sql(self)
    }
}

impl ToSql for TankSnapshot {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        serializable_to_sql(self)
    }
}

impl FromSql for BasicAccountInfo {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        deserializable_from_sql(value)
    }
}

fn serializable_to_sql<T: Serialize>(object: &T) -> rusqlite::Result<ToSqlOutput<'_>> {
    Ok(ToSqlOutput::from(serde_json::to_string(object).map_err(
        |error| rusqlite::Error::ToSqlConversionFailure(error.into()),
    )?))
}

fn deserializable_from_sql<T: DeserializeOwned>(value: ValueRef<'_>) -> FromSqlResult<T> {
    serde_json::from_str(value.as_str()?).map_err(|error| FromSqlError::Other(error.into()))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn open_database_ok() -> crate::Result {
        Database::open(":memory:")?;
        Ok(())
    }

    #[test]
    fn upsert_account_ok() -> crate::Result {
        let info = BasicAccountInfo {
            id: 42,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        };

        let database = Database::open(":memory:")?;
        database.upsert_account(&info)?;
        database.upsert_account(&info)?;
        assert_eq!(database.get_account_count()?, 1);
        Ok(())
    }

    #[test]
    fn commit_transaction() -> crate::Result {
        let info = BasicAccountInfo {
            id: 42,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        };
        let database = Database::open(":memory:")?;
        let tx = database.transaction()?;
        database.upsert_account(&info)?;
        tx.commit()?;
        assert_eq!(database.get_account_count()?, 1);
        Ok(())
    }

    #[test]
    fn drop_transaction() -> crate::Result {
        let info = BasicAccountInfo {
            id: 42,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        };
        let database = Database::open(":memory:")?;
        {
            let _tx = database.transaction()?;
            database.upsert_account(&info)?;
        }
        assert_eq!(database.get_account_count()?, 0);
        Ok(())
    }
}
