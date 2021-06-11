use std::ops::Deref;
use std::sync::{Arc, Mutex as SyncMutex};

use anyhow::anyhow;
use async_std::task::spawn_blocking;
use sqlite::{Bindable, Connection, Readable, State, Statement};

use crate::models::{AccountInfo, BasicAccountInfo, Tank};
use std::time::Instant;

/// Convenience collection container.
#[derive(Clone)]
pub struct Database {
    inner: Arc<SyncMutex<Connection>>,
}

impl Database {
    /// Open and initialize the database.
    pub async fn open<P: Into<String>>(path: P) -> crate::Result<Self> {
        let path = path.into();

        log::info!("Connecting to the database…");
        let mut connection = spawn_blocking(move || Connection::open(&path)).await?;

        log::info!("Initializing the database…");
        connection.set_busy_timeout(5000)?;
        let database = Self {
            inner: Arc::new(SyncMutex::new(connection)),
        };

        log::info!("Initializing the database schema…");
        // language=SQL
        database
            .on_inner(|inner| {
                Ok(inner.execute(
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
                )?)
            })
            .await?;

        Ok(database)
    }

    pub async fn get_account_count(&self) -> crate::Result<i64> {
        self.on_inner(|inner| {
            // language=SQL
            Self::read_scalar(inner.prepare("SELECT count(*) FROM accounts;")?)
        })
        .await
    }

    pub async fn get_account_snapshot_count(&self) -> crate::Result<i64> {
        self.on_inner(|inner| {
            // language=SQL
            Self::read_scalar(inner.prepare("SELECT count(*) FROM account_snapshots;")?)
        })
        .await
    }

    pub async fn get_tank_snapshot_count(&self) -> crate::Result<i64> {
        self.on_inner(|inner| {
            // language=SQL
            Self::read_scalar(inner.prepare("SELECT count(*) FROM tank_snapshots;")?)
        })
        .await
    }

    pub async fn get_oldest_account(&self) -> crate::Result<Option<BasicAccountInfo>> {
        let document: String = self.on_inner(|inner| {
            // language=SQL
            Self::read_scalar(inner.prepare(r"SELECT document FROM accounts ORDER BY json_extract(document, '$.crawled_at') LIMIT 1;")?)
        }).await?;
        Ok(serde_json::from_str(&document)?)
    }

    pub async fn upsert_account(&self, info: &BasicAccountInfo) -> crate::Result {
        let document = serde_json::to_string(info)?;
        self.on_inner(move |inner| {
            // language=SQL
            let mut statement =
                inner.prepare("INSERT OR REPLACE INTO accounts (document) VALUES (?)")?;
            Self::write_scalar(&mut statement, document.as_str())
        })
        .await
    }

    pub async fn upsert_account_snapshot(&self, info: &AccountInfo) -> crate::Result {
        let document = serde_json::to_string(info)?;
        self.on_inner(move |inner| {
            // language=SQL
            let mut statement =
                inner.prepare("INSERT OR IGNORE INTO account_snapshots (document) VALUES (?)")?;
            Self::write_scalar(&mut statement, document.as_str())
        })
        .await
    }

    pub async fn upsert_tanks(&self, tanks: &[Tank]) -> crate::Result {
        let documents = tanks
            .iter()
            .map(serde_json::to_string)
            .collect::<serde_json::Result<Vec<String>>>()?;
        self.on_inner(move |inner| {
            let start_instant = Instant::now();
            // language=SQL
            let mut statement =
                inner.prepare("INSERT OR IGNORE INTO tank_snapshots (document) VALUES (?)")?;
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
        })
        .await
    }

    async fn on_inner<F, T>(&self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&Connection) -> crate::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let inner = self.inner.clone();
        spawn_blocking(move || {
            f(inner
                .lock()
                .map_err(|error| anyhow!("failed to lock the database: {}", error))?
                .deref())
        })
        .await
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[async_std::test]
    async fn test_open() -> crate::Result {
        Database::open(":memory:").await?;
        Ok(())
    }

    #[async_std::test]
    async fn test_upsert_account() -> crate::Result {
        let info = BasicAccountInfo {
            id: 42,
            last_battle_time: Utc::now(),
            crawled_at: Utc::now(),
        };

        let database = Database::open(":memory:").await?;
        database.upsert_account(&info).await?;
        database.upsert_account(&info).await?;
        assert_eq!(database.get_account_count().await?, 1);
        Ok(())
    }
}
