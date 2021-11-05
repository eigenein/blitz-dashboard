use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration as StdDuration;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use log::LevelFilter;
use rocket::log::private::Level;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::{ConnectOptions, Error, Executor, FromRow, PgConnection, PgPool, Row};

use crate::metrics::Stopwatch;
use crate::models::{
    BaseAccountInfo, BaseTankStatistics, Statistics, Tank, TankAchievements, TankStatistics,
};

/// Open and initialize the database.
#[tracing::instrument(skip(uri), err)]
pub async fn open(uri: &str, initialize_schema: bool) -> crate::Result<PgPool> {
    tracing::info!("connecting…");
    let mut options = PgConnectOptions::from_str(uri)?;
    options.log_statements(LevelFilter::Trace);
    options.log_slow_statements(LevelFilter::Warn, StdDuration::from_secs(1));
    let inner = PgPoolOptions::new()
        .connect_with(options)
        .await
        .context("failed to connect")?;

    if initialize_schema {
        tracing::info!("initializing the schema…");
        inner
            .execute(include_str!("database/script.sql"))
            .await
            .context("failed to run the script")?;
    }

    tracing::info!("ready");
    Ok(inner)
}

pub async fn retrieve_latest_tank_snapshots(
    connection: &PgPool,
    account_id: i32,
    before: &DateTime<Utc>,
) -> crate::Result<HashMap<i32, Tank>> {
    // language=SQL
    const QUERY: &str = "
        SELECT snapshot.*
        FROM vehicles vehicle
        CROSS JOIN LATERAL (
            SELECT * FROM tank_snapshots snapshot
            WHERE
                snapshot.account_id = $1
                AND snapshot.tank_id = vehicle.tank_id
                AND snapshot.last_battle_time <= $2
            ORDER BY snapshot.last_battle_time DESC
            LIMIT 1
        ) snapshot
    ";

    let _stopwatch = Stopwatch::new(format!(
        "Retrieved latest tank snapshots for #{}",
        account_id,
    ))
    .threshold_millis(250)
    .level(Level::Debug);
    let tanks = sqlx::query_as(QUERY)
        .bind(account_id)
        .bind(before)
        .fetch_all(connection)
        .await
        .context("failed to retrieve the latest tank snapshots")?
        .into_iter()
        .map(|tank: Tank| (tank.statistics.base.tank_id, tank))
        .collect();
    Ok(tanks)
}

pub async fn retrieve_tank_battle_count(
    connection: &PgPool,
    account_id: i32,
    tank_id: i32,
) -> crate::Result<(i32, i32)> {
    // language=SQL
    const QUERY: &str = "
        SELECT battles, wins
        FROM tank_snapshots
        WHERE account_id = $1 AND tank_id = $2
        ORDER BY last_battle_time DESC 
        LIMIT 1
    ";
    Ok(sqlx::query_as(QUERY)
        .bind(account_id)
        .bind(tank_id)
        .fetch_optional(connection)
        .await
        .context("failed to retrieve tank battle count")?
        .unwrap_or((0, 0)))
}

pub async fn replace_account(
    connection: &mut PgConnection,
    account: BaseAccountInfo,
) -> crate::Result {
    // language=SQL
    const QUERY: &str = "
        INSERT INTO accounts (account_id, last_battle_time)
        VALUES ($1, $2)
        ON CONFLICT (account_id) DO UPDATE SET
            last_battle_time = excluded.last_battle_time
    ";
    let account_id = account.id;
    sqlx::query(QUERY)
        .bind(account.id)
        .bind(account.last_battle_time)
        .execute(connection)
        .await
        .with_context(|| format!("failed to replace the account #{}", account_id))?;
    Ok(())
}

pub async fn insert_account_if_not_exists(connection: &PgPool, account_id: i32) -> crate::Result {
    // language=SQL
    const QUERY: &str = "
        INSERT INTO accounts (account_id, last_battle_time)
        VALUES ($1, TIMESTAMP WITH TIME ZONE '1970-01-01 00:00:00+00')
        ON CONFLICT (account_id) DO NOTHING
    ";
    sqlx::query(QUERY)
        .bind(account_id)
        .execute(connection)
        .await
        .context("failed to insert the account if not exists")?;
    Ok(())
}

pub async fn insert_tank_snapshots(connection: &mut PgConnection, tanks: &[Tank]) -> crate::Result {
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
        log::trace!(
            "Inserting #{}/#{} tank snapshot…",
            snapshot.account_id,
            snapshot.statistics.base.tank_id
        );
        sqlx::query(QUERY)
            .bind(snapshot.account_id)
            .bind(snapshot.statistics.base.tank_id)
            .bind(snapshot.statistics.base.last_battle_time)
            .bind(snapshot.statistics.battle_life_time.num_seconds())
            .bind(snapshot.statistics.all.battles)
            .bind(snapshot.statistics.all.wins)
            .bind(snapshot.statistics.all.survived_battles)
            .bind(snapshot.statistics.all.win_and_survived)
            .bind(snapshot.statistics.all.damage_dealt)
            .bind(snapshot.statistics.all.damage_received)
            .bind(snapshot.statistics.all.shots)
            .bind(snapshot.statistics.all.hits)
            .bind(snapshot.statistics.all.frags)
            .bind(snapshot.statistics.all.xp)
            .execute(&mut *connection)
            .await
            .context("failed to insert tank snapshots")?;
    }
    Ok(())
}

pub async fn retrieve_random_account_id(connection: &PgPool) -> crate::Result<Option<i32>> {
    // language=SQL
    const QUERY: &str = r#"SELECT account_id FROM accounts LIMIT 1"#;
    let account_id = sqlx::query_scalar(QUERY)
        .fetch_optional(connection)
        .await
        .context("failed to retrieve a random account ID")?;
    Ok(account_id)
}

pub async fn insert_vehicle_or_ignore(
    connection: &mut PgConnection,
    tank_id: i32,
) -> crate::Result {
    // language=SQL
    const QUERY: &str = "
        INSERT INTO vehicles (tank_id)
        VALUES ($1)
        ON CONFLICT (tank_id) DO NOTHING
    ";
    sqlx::query(QUERY)
        .bind(tank_id)
        .execute(connection)
        .await
        .context("failed to insert the vehicle or ignore")?;
    Ok(())
}

pub async fn retrieve_tank_ids(connection: &PgPool) -> crate::Result<Vec<i32>> {
    // language=SQL
    const QUERY: &str = "SELECT tank_id FROM vehicles";
    Ok(sqlx::query_scalar(QUERY)
        .fetch_all(connection)
        .await
        .context("failed to retrieve all tank IDs")?)
}

impl<'r> FromRow<'r, PgRow> for Tank {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            account_id: row.try_get("account_id")?,
            statistics: TankStatistics::from_row(row)?,
            achievements: TankAchievements::from_row(row)?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for TankStatistics {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        let battle_life_time: i64 = row.try_get("battle_life_time")?;
        Ok(Self {
            base: BaseTankStatistics::from_row(row)?,
            battle_life_time: Duration::seconds(battle_life_time),
            all: Statistics::from_row(row)?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for BaseTankStatistics {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        Ok(Self {
            tank_id: row.try_get("tank_id")?,
            last_battle_time: row.try_get("last_battle_time")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for TankAchievements {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        Ok(Self {
            tank_id: row.try_get("tank_id")?,
            achievements: Default::default(), // TODO
            max_series: Default::default(),   // TODO
        })
    }
}

impl<'r> FromRow<'r, PgRow> for BaseAccountInfo {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("account_id")?,
            last_battle_time: row.try_get("last_battle_time")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for Statistics {
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
