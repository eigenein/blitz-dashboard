use anyhow::anyhow;
use sqlite::{Bindable, Connection, Readable, State, Statement};

use crate::logging::log_anyhow;
use crate::models::{AccountInfo, BasicAccountInfo, Tank};
use std::time::Instant;

pub struct Database {
    inner: Connection,
}

impl Database {
    /// Open and initialize the database.
    pub fn open<P: Into<String>>(path: P) -> crate::Result<Self> {
        let path = path.into();

        log::info!("Connecting to the database…");
        let mut inner = Connection::open(&path)?;

        log::info!("Initializing the database…");
        inner.set_busy_timeout(5000)?;

        log::info!("Initializing the database schema…");
        // language=SQL
        inner.execute(
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

        Ok(Self { inner })
    }

    pub fn transaction(&self) -> crate::Result<Transaction> {
        // language=SQL
        self.inner.execute("BEGIN IMMEDIATE")?;
        Ok(Transaction {
            inner: &self.inner,
            is_committed: false,
        })
    }

    pub fn get_account_count(&self) -> crate::Result<i64> {
        // language=SQL
        Self::read_scalar(self.inner.prepare("SELECT count(*) FROM accounts;")?)
    }

    pub fn get_account_snapshot_count(&self) -> crate::Result<i64> {
        // language=SQL
        Self::read_scalar(
            self.inner
                .prepare("SELECT count(*) FROM account_snapshots;")?,
        )
    }

    pub fn get_tank_snapshot_count(&self) -> crate::Result<i64> {
        // language=SQL
        Self::read_scalar(
            self.inner
                .prepare("SELECT count(*) FROM account_snapshots;")?,
        )
    }

    pub fn get_oldest_account(&self) -> crate::Result<Option<BasicAccountInfo>> {
        // language=SQL
        let document: String = Self::read_scalar(self.inner.prepare(r"SELECT document FROM accounts ORDER BY json_extract(document, '$.crawled_at') LIMIT 1;")?)?;
        Ok(serde_json::from_str(&document)?)
    }

    pub fn upsert_account(&self, info: &BasicAccountInfo) -> crate::Result {
        let document = serde_json::to_string(info)?;
        // language=SQL
        let mut statement = self
            .inner
            .prepare("INSERT OR REPLACE INTO accounts (document) VALUES (?)")?;
        Self::write_scalar(&mut statement, document.as_str())
    }

    pub fn upsert_account_snapshot(&self, info: &AccountInfo) -> crate::Result {
        let document = serde_json::to_string(info)?;
        // language=SQL
        let mut statement = self
            .inner
            .prepare("INSERT OR IGNORE INTO account_snapshots (document) VALUES (?)")?;
        Self::write_scalar(&mut statement, document.as_str())
    }

    pub fn upsert_tanks(&self, tanks: &[Tank]) -> crate::Result {
        let documents = tanks
            .iter()
            .map(serde_json::to_string)
            .collect::<serde_json::Result<Vec<String>>>()?;
        let start_instant = Instant::now();
        // language=SQL
        let mut statement = self
            .inner
            .prepare("INSERT OR IGNORE INTO tank_snapshots (document) VALUES (?)")?;
        for document in documents.iter() {
            statement.reset()?;
            Self::write_scalar(&mut statement, document.as_str())?;
        }
        log::debug!(
            "{} tanks upserted in {:?}.",
            documents.len(),
            Instant::now() - start_instant,
        );
        Ok(())
    }

    fn read_scalar<T: Readable>(mut statement: Statement) -> crate::Result<T> {
        match statement.next()? {
            State::Row => Ok(statement.read(0)?),
            _ => Err(anyhow!("no results")),
        }
    }

    fn write_scalar<T: Bindable>(statement: &mut Statement, scalar: T) -> crate::Result {
        statement.bind(1, scalar)?;
        while statement.next()? != State::Done {}
        Ok(())
    }
}

pub struct Transaction<'c> {
    inner: &'c Connection,
    is_committed: bool,
}

impl Transaction<'_> {
    pub fn commit(mut self) -> crate::Result {
        // language=SQL
        self.inner.execute("COMMIT")?;
        self.is_committed = true;
        Ok(())
    }
}

impl<'c> Drop for Transaction<'c> {
    fn drop(&mut self) {
        if !self.is_committed {
            log::error!("Dropped transaction, rolling back.");
            // language=SQL
            log_anyhow(self.inner.execute("ROLLBACK").map_err(anyhow::Error::from));
        }
    }
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
