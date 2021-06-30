use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use rusqlite::{params, OptionalExtension, Row, ToSql};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::metrics::Stopwatch;
use crate::models::{AccountInfo, BasicAccountInfo, TankSnapshot, Vehicle};

pub struct Database(rusqlite::Connection);

pub struct Statistics {
    pub account_count: i64,
    pub account_snapshot_count: i64,
    pub tank_snapshot_count: i64,
}

impl Database {
    /// Open and initialize the database.
    pub fn open<P: Into<String>>(path: P) -> crate::Result<Self> {
        let path = path.into();

        log::info!("Connecting to the database…");
        let inner = rusqlite::Connection::open(&path)?;
        inner.busy_timeout(StdDuration::from_secs(5))?;

        log::info!("Initializing the database schema…");
        inner.execute_batch(SCRIPT)?;

        Ok(Self(inner))
    }

    pub fn start_transaction(&self) -> crate::Result<Transaction> {
        Ok(Transaction(self.0.unchecked_transaction()?))
    }

    pub fn retrieve_account_count(&self) -> crate::Result<i64> {
        Ok(self
            .0
            .prepare_cached(
                // language=SQL
                "SELECT count(*) FROM accounts",
            )?
            .query_row([], get_scalar)?)
    }

    pub fn retrieve_account_snapshot_count(&self) -> crate::Result<i64> {
        Ok(self
            .0
            .prepare_cached(
                // language=SQL
                "SELECT count(*) FROM account_snapshots",
            )?
            .query_row([], get_scalar)?)
    }

    pub fn retrieve_tank_snapshot_count(&self) -> crate::Result<i64> {
        Ok(self
            .0
            .prepare_cached(
                // language=SQL
                "SELECT count(*) FROM tank_snapshots",
            )?
            .query_row([], get_scalar)?)
    }

    pub fn retrieve_oldest_accounts(&self, limit: i32) -> crate::Result<Vec<BasicAccountInfo>> {
        Ok(self
            .0
            // language=SQL
            .prepare_cached("SELECT document FROM accounts ORDER BY json_extract(document, '$.crawled_at') LIMIT ?1")?
            .query_map([limit], get_scalar)?
            .collect::<rusqlite::Result<Vec<BasicAccountInfo>>>()?
        )
    }

    pub fn retrieve_latest_account_snapshot(
        &self,
        account_id: i32,
        before: &DateTime<Utc>,
    ) -> crate::Result<Option<AccountInfo>> {
        Ok(self
            .0

            .prepare_cached(
                // language=SQL
                "SELECT document 
                FROM account_snapshots
                WHERE json_extract(document, '$.account_id') = ?1 AND json_extract(document, '$.last_battle_time') <= ?2
                ORDER BY json_extract(document, '$.last_battle_time') DESC
                LIMIT 1",
            )?
            .query_row(params![account_id, before.timestamp()], get_scalar)
            .optional()?)
    }

    pub fn retrieve_latest_tank_snapshots(
        &self,
        account_id: i32,
        before: &DateTime<Utc>,
    ) -> crate::Result<Vec<TankSnapshot>> {
        let _stopwatch = Stopwatch::new("Retrieved latest tank snapshots").threshold_millis(30);
        Ok(self
            .0
            .prepare_cached(
                // https://www.sqlite.org/lang_select.html#bareagg
                // language=SQL
                "SELECT document, max(json_extract(document, '$.last_battle_time'))
                FROM tank_snapshots
                WHERE json_extract(document, '$.account_id') = ?1 AND json_extract(document, '$.last_battle_time') <= ?2
                GROUP BY json_extract(document, '$.tank_id')",
            )?
            .query_map(params![account_id, before.timestamp()], get_scalar)?
            .collect::<rusqlite::Result<Vec<TankSnapshot>>>()?)
    }

    pub fn insert_account_or_replace(&self, info: &BasicAccountInfo) -> crate::Result {
        self.0
            .prepare_cached(
                // language=SQL
                "INSERT OR REPLACE INTO accounts (document) VALUES (?1)",
            )?
            .execute([info])?;
        Ok(())
    }

    pub fn insert_account_or_ignore(&self, info: &BasicAccountInfo) -> crate::Result {
        self.0
            .prepare_cached(
                // language=SQL
                "INSERT OR IGNORE INTO accounts (document) VALUES (?1)",
            )?
            .execute([info])?;
        Ok(())
    }

    /// Deletes all data related to the account.
    pub fn prune_account(&self, account_id: i32) -> crate::Result {
        self.delete_account(account_id)?;
        self.delete_account_snapshots(account_id)?;
        self.delete_tank_snapshots(account_id)?;
        Ok(())
    }

    pub fn delete_account(&self, account_id: i32) -> crate::Result {
        self.0
            .prepare_cached(
                // language=SQL
                "DELETE FROM accounts WHERE json_extract(document, '$.account_id') = ?1",
            )?
            .execute([account_id])?;
        Ok(())
    }

    pub fn delete_account_snapshots(&self, account_id: i32) -> crate::Result {
        self.0
            .prepare_cached(
                // language=SQL
                "DELETE FROM account_snapshots WHERE json_extract(document, '$.account_id') = ?1",
            )?
            .execute([account_id])?;
        Ok(())
    }

    pub fn delete_tank_snapshots(&self, account_id: i32) -> crate::Result {
        self.0
            .prepare_cached(
                // language=SQL
                "DELETE FROM tank_snapshots WHERE json_extract(document, '$.account_id') = ?1",
            )?
            .execute([account_id])?;
        Ok(())
    }

    pub fn upsert_account_snapshot(&self, info: &AccountInfo) -> crate::Result {
        log::info!("Upserting account #{} snapshot…", info.basic.id);
        self.0
            .prepare_cached(
                // language=SQL
                "INSERT OR REPLACE INTO account_snapshots (document) VALUES (?1)",
            )?
            .execute([info])?;
        Ok(())
    }

    pub fn upsert_tank_snapshots(&self, snapshots: &[TankSnapshot]) -> crate::Result {
        log::info!("Upserting {} tank snapshots…", snapshots.len());
        let mut statement = self
            .0
            // language=SQL
            .prepare_cached("INSERT OR IGNORE INTO tank_snapshots (document) VALUES (?1)")?;
        for snapshot in snapshots {
            log::debug!(
                "Upserting #{}/#{} tank snapshot…",
                snapshot.account_id,
                snapshot.tank_id
            );
            statement.execute(&[snapshot])?;
        }
        Ok(())
    }

    pub fn upsert_vehicles(&self, vehicles: &[Vehicle]) -> crate::Result {
        log::info!("Upserting {} vehicles…", vehicles.len());
        let mut statement = self.0.prepare_cached(
            // language=SQL
            "INSERT OR REPLACE INTO tankopedia (document) VALUES (?1)",
        )?;
        for vehicle in vehicles {
            statement.execute(&[vehicle])?;
        }
        log::info!("Upserted {} vehicles.", vehicles.len());
        Ok(())
    }

    pub fn retrieve_vehicle(&self, tank_id: i32) -> crate::Result<Option<Vehicle>> {
        let _stopwatch = Stopwatch::new("Retrieved tank").threshold_millis(1);
        Ok(self
            .0
            .prepare_cached(
                // language=SQL
                "SELECT document FROM tankopedia WHERE json_extract(document, '$.tank_id') = ?1",
            )?
            .query_row([tank_id], get_scalar)
            .optional()?)
    }

    pub fn retrieve_statistics(&self) -> crate::Result<Statistics> {
        Ok(Statistics {
            account_count: self.retrieve_account_count()?,
            account_snapshot_count: self.retrieve_account_snapshot_count()?,
            tank_snapshot_count: self.retrieve_tank_snapshot_count()?,
        })
    }
}

#[inline]
fn get_scalar<T: FromSql>(row: &Row) -> rusqlite::Result<T> {
    row.get(0)
}

pub struct Transaction<'c>(rusqlite::Transaction<'c>);

impl Transaction<'_> {
    pub fn commit(self) -> crate::Result {
        Ok(self.0.commit()?)
    }
}

// language=SQL
const SCRIPT: &str = r#"
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

    CREATE TABLE IF NOT EXISTS tankopedia (document JSON NOT NULL);
    CREATE UNIQUE INDEX IF NOT EXISTS tankopedia_tank_id
        ON tankopedia(json_extract(document, '$.tank_id'));

    VACUUM;
"#;

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

impl ToSql for Vehicle {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        serializable_to_sql(self)
    }
}

impl FromSql for BasicAccountInfo {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        deserializable_from_sql(value)
    }
}

impl FromSql for Vehicle {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        deserializable_from_sql(value)
    }
}

impl FromSql for AccountInfo {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        deserializable_from_sql(value)
    }
}

impl FromSql for TankSnapshot {
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
    use std::collections::HashMap;

    use chrono::{Duration, TimeZone, Utc};

    use crate::models::AllStatistics;

    use super::*;

    #[test]
    fn open_database_ok() -> crate::Result {
        Database::open(":memory:")?;
        Ok(())
    }

    #[test]
    fn insert_account_or_replace_ok() -> crate::Result {
        let info = BasicAccountInfo {
            id: 42,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        };

        let database = Database::open(":memory:")?;
        database.insert_account_or_replace(&info)?;
        database.insert_account_or_replace(&info)?;
        assert_eq!(database.retrieve_account_count()?, 1);
        Ok(())
    }

    #[test]
    fn delete_account_ok() -> crate::Result {
        let database = Database::open(":memory:")?;
        database.insert_account_or_replace(&BasicAccountInfo {
            id: 1,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        })?;
        database.insert_account_or_replace(&BasicAccountInfo {
            id: 2,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        })?;
        database.delete_account(1)?;
        assert_eq!(database.retrieve_account_count()?, 1);
        Ok(())
    }

    #[test]
    fn retrieve_latest_tank_snaphots_ok() -> crate::Result {
        let database = Database::open(":memory:")?;

        database.upsert_tank_snapshots(&[
            TankSnapshot {
                account_id: 1,
                tank_id: 42,
                achievements: HashMap::new(),
                max_series: HashMap::new(),
                all_statistics: AllStatistics {
                    battles: 1,
                    ..Default::default()
                },
                last_battle_time: Utc.timestamp(1, 0),
                battle_life_time: Duration::seconds(1),
            },
            TankSnapshot {
                account_id: 1,
                tank_id: 42,
                achievements: HashMap::new(),
                max_series: HashMap::new(),
                all_statistics: AllStatistics {
                    battles: 2,
                    ..Default::default()
                },
                last_battle_time: Utc.timestamp(2, 0),
                battle_life_time: Duration::seconds(1),
            },
        ])?;

        let snapshots = database.retrieve_latest_tank_snapshots(1, &Utc.timestamp(2, 0))?;
        assert_eq!(snapshots.len(), 1);
        let snapshot = snapshots.get(0).unwrap();
        assert_eq!(snapshot.last_battle_time, Utc.timestamp(2, 0));
        assert_eq!(snapshot.all_statistics.battles, 2);

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
        let tx = database.start_transaction()?;
        database.insert_account_or_replace(&info)?;
        tx.commit()?;
        assert_eq!(database.retrieve_account_count()?, 1);
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
            let _tx = database.start_transaction()?;
            database.insert_account_or_replace(&info)?;
        }
        assert_eq!(database.retrieve_account_count()?, 0);
        Ok(())
    }
}
