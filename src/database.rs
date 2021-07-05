use std::str::FromStr;
use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use log::LevelFilter;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::{ConnectOptions, Executor, FromRow, PgPool, Postgres, Row, Transaction};

use crate::metrics::Stopwatch;
use crate::models::{
    AccountInfo, AccountInfoStatistics, AllStatistics, BasicAccountInfo, Nation, TankSnapshot,
    TankType, Vehicle,
};

pub struct Statistics {
    pub account_count: i64,
    pub account_snapshot_count: i64,
    pub tank_snapshot_count: i64,
}

/// Open and initialize the database.
pub async fn open(uri: &str) -> crate::Result<PgPool> {
    log::info!("Connecting to the database…");
    let mut options = PgConnectOptions::from_str(uri)?;
    options.log_statements(LevelFilter::Trace);
    options.log_slow_statements(LevelFilter::Warn, StdDuration::from_secs(1));
    let inner = PgPoolOptions::new()
        .connect_with(options)
        .await
        .context("failed to connect")?;

    log::info!("Initializing the database schema…");
    inner
        .execute(SCRIPT)
        .await
        .context("failed to run the script")?;

    Ok(inner)
}

pub async fn retrieve_account_count(executor: &PgPool) -> crate::Result<i64> {
    // language=SQL
    const QUERY: &str = "SELECT count(*) FROM accounts";
    Ok(sqlx::query_scalar(QUERY)
        .fetch_one(executor)
        .await
        .context("failed to select count from accounts")?)
}

pub async fn retrieve_account_snapshot_count(executor: &PgPool) -> crate::Result<i64> {
    // language=SQL
    const QUERY: &str = "SELECT count(*) FROM account_snapshots";
    Ok(sqlx::query_scalar(QUERY)
        .fetch_one(executor)
        .await
        .context("failed to select count from account snapshots")?)
}

pub async fn retrieve_tank_snapshot_count(executor: &PgPool) -> crate::Result<i64> {
    // language=SQL
    const QUERY: &str = "SELECT count(*) FROM tank_snapshots";
    Ok(sqlx::query_scalar(QUERY)
        .fetch_one(executor)
        .await
        .context("failed to select count from tank snapshots")?)
}

pub async fn retrieve_oldest_accounts(
    executor: &PgPool,
    limit: i32,
) -> crate::Result<Vec<BasicAccountInfo>> {
    // language=SQL
    const QUERY: &str = "
        SELECT * FROM accounts
        ORDER BY crawled_at NULLS FIRST
        LIMIT $1
    ";
    let accounts = sqlx::query_as(QUERY)
        .bind(limit)
        .fetch_all(executor)
        .await
        .context("failed to retrieve the oldest accounts")?;
    Ok(accounts)
}

pub async fn retrieve_latest_account_snapshot(
    executor: &PgPool,
    account_id: i32,
    before: &DateTime<Utc>,
) -> crate::Result<Option<AccountInfo>> {
    // language=SQL
    const QUERY: &str = "
        SELECT *
        FROM account_snapshots
        WHERE account_id = $1 AND last_battle_time <= $2
        ORDER BY last_battle_time DESC
        LIMIT 1
    ";
    let account_info = sqlx::query_as(QUERY)
        .bind(account_id)
        .bind(before)
        .fetch_optional(executor)
        .await
        .context("failed to retrieve the latest account snapshot")?;
    Ok(account_info)
}

pub async fn retrieve_latest_tank_snapshots(
    executor: &PgPool,
    account_id: i32,
    before: &DateTime<Utc>,
) -> crate::Result<Vec<TankSnapshot>> {
    // language=SQL
    const QUERY: &str = "
        SELECT DISTINCT ON (tank_id) *
        FROM tank_snapshots
        WHERE account_id = $1 AND last_battle_time <= $2
        ORDER BY tank_id, last_battle_time DESC
    ";

    let _stopwatch = Stopwatch::new("Retrieved latest tank snapshots").threshold_millis(30);
    Ok(sqlx::query_as(QUERY)
        .bind(account_id)
        .bind(before)
        .fetch_all(executor)
        .await
        .context("failed to retrieve the latest tank snapshots")?)
}

pub async fn insert_account_or_replace<'e, E: Executor<'e, Database = Postgres>>(
    executor: E,
    info: &BasicAccountInfo,
) -> crate::Result {
    // language=SQL
    const QUERY: &str = "
        INSERT INTO accounts (account_id, last_battle_time, crawled_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (account_id) DO UPDATE SET
            last_battle_time = EXCLUDED.last_battle_time,
            crawled_at = EXCLUDED.crawled_at
    ";
    sqlx::query(QUERY)
        .bind(info.id)
        .bind(info.last_battle_time)
        .bind(info.crawled_at)
        .execute(executor)
        .await
        .context("failed to insert the account or replace")?;
    Ok(())
}

pub async fn insert_account_or_ignore(executor: &PgPool, info: &BasicAccountInfo) -> crate::Result {
    // language=SQL
    const QUERY: &str = "
        INSERT INTO accounts (account_id, last_battle_time, crawled_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (account_id) DO NOTHING
    ";
    sqlx::query(QUERY)
        .bind(info.id)
        .bind(info.last_battle_time)
        .bind(info.crawled_at)
        .execute(executor)
        .await
        .context("failed to insert the account or ignore")?;
    Ok(())
}

pub async fn delete_account<'e, E>(executor: E, account_id: i32) -> crate::Result
where
    E: Executor<'e, Database = Postgres>,
{
    // language=SQL
    const QUERY: &str = "DELETE FROM accounts WHERE account_id = $1";
    sqlx::query(QUERY)
        .bind(account_id)
        .execute(executor)
        .await
        .context("failed to delete account")?;
    Ok(())
}

pub async fn insert_account_snapshot(
    executor: &mut Transaction<'_, Postgres>,
    info: &AccountInfo,
) -> crate::Result {
    log::info!("Inserting account #{} snapshot…", info.basic.id);
    // language=SQL
    const QUERY: &str = "
        INSERT INTO account_snapshots (
            account_id,
            last_battle_time,
            crawled_at,
            battles,
            wins,
            survived_battles,
            win_and_survived,
            damage_dealt,
            damage_received,
            shots,
            hits,
            frags,
            xp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (account_id, last_battle_time) DO NOTHING
    ";
    sqlx::query(QUERY)
        .bind(info.basic.id)
        .bind(info.basic.last_battle_time)
        .bind(info.basic.crawled_at)
        .bind(info.statistics.all.battles)
        .bind(info.statistics.all.wins)
        .bind(info.statistics.all.survived_battles)
        .bind(info.statistics.all.win_and_survived)
        .bind(info.statistics.all.damage_dealt)
        .bind(info.statistics.all.damage_received)
        .bind(info.statistics.all.shots)
        .bind(info.statistics.all.hits)
        .bind(info.statistics.all.frags)
        .bind(info.statistics.all.xp)
        .execute(executor)
        .await
        .context("failed to insert account snapshot")?;
    Ok(())
}

pub async fn insert_tank_snapshots(
    executor: &mut Transaction<'_, Postgres>,
    snapshots: &[TankSnapshot],
) -> crate::Result {
    log::info!("Inserting {} tank snapshots…", snapshots.len());
    let _stopwatch = Stopwatch::new("Inserted in").threshold_millis(1000);

    // language=SQL
    const QUERY: &str = "
        INSERT INTO tank_snapshots (
            account_id,
            tank_id,
            last_battle_time,
            battle_life_time,
            battles,
            wins,
            survived_battles,
            win_and_survived,
            damage_dealt,
            damage_received,
            shots,
            hits,
            frags,
            xp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (account_id, tank_id, last_battle_time) DO NOTHING
    ";
    for snapshot in snapshots {
        log::debug!(
            "Inserting #{}/#{} tank snapshot…",
            snapshot.account_id,
            snapshot.tank_id
        );
        sqlx::query(QUERY)
            .bind(snapshot.account_id)
            .bind(snapshot.tank_id)
            .bind(snapshot.last_battle_time)
            .bind(snapshot.battle_life_time.num_seconds())
            .bind(snapshot.all_statistics.battles)
            .bind(snapshot.all_statistics.wins)
            .bind(snapshot.all_statistics.survived_battles)
            .bind(snapshot.all_statistics.win_and_survived)
            .bind(snapshot.all_statistics.damage_dealt)
            .bind(snapshot.all_statistics.damage_received)
            .bind(snapshot.all_statistics.shots)
            .bind(snapshot.all_statistics.hits)
            .bind(snapshot.all_statistics.frags)
            .bind(snapshot.all_statistics.xp)
            .execute(&mut *executor)
            .await
            .context("failed to insert tank snapshots")?;
    }
    Ok(())
}

pub async fn insert_vehicles(
    executor: &mut Transaction<'_, Postgres>,
    vehicles: &[Vehicle],
) -> crate::Result {
    // language=SQL
    const QUERY: &str = r#"
        INSERT INTO tankopedia (tank_id, "name", tier, is_premium, nation, "type")
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (tank_id) DO UPDATE SET
            "name" = EXCLUDED."name",
            "is_premium" = EXCLUDED.is_premium,
            "type" = EXCLUDED."type"
    "#;

    log::info!("Inserting {} vehicles…", vehicles.len());
    for vehicle in vehicles {
        sqlx::query(QUERY)
            .bind(vehicle.tank_id)
            .bind(vehicle.name.clone())
            .bind(vehicle.tier)
            .bind(vehicle.is_premium)
            .bind(vehicle.nation.to_string())
            .bind(vehicle.type_.to_string())
            .execute(&mut *executor)
            .await
            .context("failed to insert the vehicles")?;
    }
    log::info!("Inserted {} vehicles.", vehicles.len());
    Ok(())
}

pub async fn retrieve_vehicles(executor: &PgPool) -> crate::Result<Vec<Vehicle>> {
    Ok(sqlx::query_as(
        // language=SQL
        "SELECT * FROM tankopedia",
    )
    .fetch_all(executor)
    .await
    .context("failed to retrieve the tankopedia")?)
}

pub async fn retrieve_statistics(executor: &PgPool) -> crate::Result<Statistics> {
    let account_count = retrieve_account_count(&executor).await?;
    let account_snapshot_count = retrieve_account_snapshot_count(&executor).await?;
    let tank_snapshot_count = retrieve_tank_snapshot_count(&executor).await?;
    Ok(Statistics {
        account_count,
        account_snapshot_count,
        tank_snapshot_count,
    })
}

impl<'r> FromRow<'r, PgRow> for Vehicle {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            tank_id: row.try_get("tank_id")?,
            name: row.try_get("name")?,
            tier: row.try_get("tier")?,
            is_premium: row.try_get("is_premium")?,
            nation: Nation::from_str(&row.try_get::<String, _>("nation")?).map_err(|error| {
                sqlx::Error::ColumnDecode {
                    index: "nation".to_string(),
                    source: error.into(),
                }
            })?,
            type_: TankType::from_str(&row.try_get::<String, _>("type")?).map_err(|error| {
                sqlx::Error::ColumnDecode {
                    index: "type".to_string(),
                    source: error.into(),
                }
            })?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for TankSnapshot {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let battle_life_time: i64 = row.try_get("battle_life_time")?;
        Ok(Self {
            account_id: row.try_get("account_id")?,
            tank_id: row.try_get("tank_id")?,
            last_battle_time: row.try_get("last_battle_time")?,
            battle_life_time: Duration::seconds(battle_life_time),
            all_statistics: AllStatistics::from_row(row)?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for AccountInfo {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            basic: BasicAccountInfo::from_row(row)?,
            nickname: "".to_string(), // FIXME
            created_at: Utc::now(),   // FIXME
            statistics: AccountInfoStatistics {
                all: AllStatistics::from_row(row)?,
            },
        })
    }
}

impl<'r> FromRow<'r, PgRow> for BasicAccountInfo {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("account_id")?,
            last_battle_time: row.try_get("last_battle_time")?,
            crawled_at: row.try_get("crawled_at")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for AllStatistics {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            battles: row.try_get("battles")?,
            wins: row.try_get("wins")?,
            survived_battles: row.try_get("survived_battles")?,
            win_and_survived: row.try_get("win_and_survived")?,
            damage_dealt: row.try_get("damage_dealt")?,
            damage_received: row.try_get("damage_received")?,
            shots: row.try_get("shots")?,
            hits: row.try_get("hits")?,
            frags: row.try_get("frags")?,
            xp: row.try_get("xp")?,
        })
    }
}

// language=SQL
const SCRIPT: &str = r#"
    CREATE TABLE IF NOT EXISTS accounts (
        account_id INTEGER PRIMARY KEY,
        last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL,
        crawled_at TIMESTAMP WITH TIME ZONE NULL
    );
    CREATE INDEX IF NOT EXISTS accounts_crawled_at ON accounts(crawled_at);

    CREATE TABLE IF NOT EXISTS account_snapshots (
        account_id INTEGER NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
        last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL,
        crawled_at TIMESTAMP WITH TIME ZONE NOT NULL,
        battles INTEGER NOT NULL,
        wins INTEGER NOT NULL,
        survived_battles INTEGER NOT NULL,
        win_and_survived INTEGER NOT NULL,
        damage_dealt INTEGER NOT NULL,
        damage_received INTEGER NOT NULL,
        shots INTEGER NOT NULL,
        hits INTEGER NOT NULL,
        frags INTEGER NOT NULL,
        xp INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS account_snapshots_key
        ON account_snapshots(account_id ASC, last_battle_time DESC);

    CREATE TABLE IF NOT EXISTS tank_snapshots (
        account_id INTEGER NOT NULL REFERENCES accounts (account_id) ON DELETE CASCADE,
        tank_id INTEGER NOT NULL,
        last_battle_time TIMESTAMP WITH TIME ZONE NOT NULL,
        battle_life_time BIGINT NOT NULL,
        battles INTEGER NOT NULL,
        wins INTEGER NOT NULL,
        survived_battles INTEGER NOT NULL,
        win_and_survived INTEGER NOT NULL,
        damage_dealt INTEGER NOT NULL,
        damage_received INTEGER NOT NULL,
        shots INTEGER NOT NULL,
        hits INTEGER NOT NULL,
        frags INTEGER NOT NULL,
        xp INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS tank_snapshots_key
        ON tank_snapshots(account_id ASC, tank_id ASC, last_battle_time DESC);

    CREATE TABLE IF NOT EXISTS tankopedia (
        tank_id INTEGER PRIMARY KEY,
        "name" TEXT NOT NULL,
        tier INTEGER NOT NULL,
        is_premium BOOLEAN NOT NULL,
        nation TEXT NOT NULL,
        "type" TEXT NOT NULL
    );
"#;
