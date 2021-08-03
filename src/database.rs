use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use log::LevelFilter;
use rocket::log::private::Level;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::{ConnectOptions, Executor, FromRow, PgConnection, PgPool, Postgres, Row};

use crate::metrics::Stopwatch;
use crate::models::{AllStatistics, BaseAccountInfo, Tank};

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
        .execute(include_str!("database/script.sql"))
        .await
        .context("failed to run the script")?;

    log::info!("The database is ready.");
    Ok(inner)
}

pub async fn retrieve_latest_tank_snapshots(
    executor: &PgPool,
    account_id: i32,
    before: &DateTime<Utc>,
) -> crate::Result<HashMap<i32, Tank>> {
    // language=SQL
    const QUERY: &str = "
        SELECT DISTINCT ON (tank_id) *
        FROM tank_snapshots
        WHERE account_id = $1 AND last_battle_time <= $2
        ORDER BY tank_id, last_battle_time DESC
    ";

    let _stopwatch = Stopwatch::new(format!(
        "Retrieved latest tank snapshots for #{}",
        account_id
    ))
    .threshold_millis(100)
    .level(Level::Debug);
    let tanks = sqlx::query_as(QUERY)
        .bind(account_id)
        .bind(before)
        .fetch_all(executor)
        .await
        .context("failed to retrieve the latest tank snapshots")?
        .into_iter()
        .map(|tank: Tank| (tank.tank_id, tank))
        .collect();
    Ok(tanks)
}

pub async fn insert_account_or_replace<'e, E: Executor<'e, Database = Postgres>>(
    executor: E,
    info: &BaseAccountInfo,
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
        .with_context(|| format!("failed to insert the account #{} or replace", info.id))?;
    Ok(())
}

pub async fn insert_account_or_ignore(executor: &PgPool, info: &BaseAccountInfo) -> crate::Result {
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

pub async fn insert_tank_snapshots(connection: &mut PgConnection, tanks: &[Tank]) -> crate::Result {
    log::info!("Inserting {} tanks…", tanks.len());
    let _stopwatch = Stopwatch::new("Inserted tanks").threshold_millis(1000);

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
    for snapshot in tanks {
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
            .execute(&mut *connection)
            .await
            .context("failed to insert tank snapshots")?;
    }
    Ok(())
}

pub async fn retrieve_random_account_id(connection: &PgPool) -> crate::Result<Option<i32>> {
    // language=SQL
    const QUERY: &str = r#"
        SELECT account_id FROM accounts
        WHERE last_battle_time >= NOW() - INTERVAL '1 hour'
        LIMIT 1
    "#;
    let account_id = sqlx::query_scalar(QUERY)
        .fetch_optional(connection)
        .await
        .context("failed to retrieve a random account ID")?;
    Ok(account_id)
}

impl<'r> FromRow<'r, PgRow> for Tank {
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

impl<'r> FromRow<'r, PgRow> for BaseAccountInfo {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("account_id")?,
            last_battle_time: row.try_get("last_battle_time")?,
            crawled_at: row.try_get("crawled_at")?,
            nickname: "".to_string(), // FIXME
            created_at: Utc::now(),   // FIXME
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
