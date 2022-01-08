use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use chrono::Duration;
use futures::{StreamExt, TryStreamExt};
use humantime::format_duration;
use itertools::Itertools;
use log::{Level, LevelFilter};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::{ConnectOptions, Error, Executor, FromRow, PgConnection, PgPool, Row};
use tracing::instrument;

use crate::metrics::Stopwatch;
use crate::models::{
    BaseAccountInfo, BaseTankStatistics, Statistics, Tank, TankAchievements, TankStatistics,
};
use crate::wargaming::tank_id::TankId;
use crate::DateTime;

/// Open and initialize the database.
#[tracing::instrument(skip(uri))]
pub async fn open(uri: &str, initialize_schema: bool) -> crate::Result<PgPool> {
    tracing::info!("connecting…");
    let mut options = PgConnectOptions::from_str(uri)?;
    options.log_statements(LevelFilter::Trace);
    options.log_slow_statements(LevelFilter::Warn, StdDuration::from_millis(500));
    let inner = PgPoolOptions::new()
        .max_connections(20)
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
    before: DateTime,
    tank_ids: &[TankId],
) -> crate::Result<HashMap<TankId, Tank>> {
    // language=SQL
    const QUERY: &str = "
        SELECT snapshot.*
        FROM UNNEST($3) external_tank_id
        CROSS JOIN LATERAL (
            SELECT * FROM tank_snapshots snapshot
            WHERE
                snapshot.account_id = $1
                AND snapshot.tank_id = external_tank_id
                AND snapshot.last_battle_time <= $2
            ORDER BY snapshot.last_battle_time DESC
            LIMIT 1
        ) snapshot
    ";

    let _stopwatch = Stopwatch::new(format!(
        "retrieved latest tank snapshots for #{}",
        account_id,
    ))
    .threshold_millis(250)
    .level(Level::Debug);
    sqlx::query(QUERY)
        .bind(account_id)
        .bind(before)
        .bind(&tank_ids.iter().map(|tank_id| *tank_id as i32).collect_vec())
        .fetch(connection)
        .map(|row| {
            let row = row?;
            sqlx::Result::Ok((try_get::<i32, _>(&row, "tank_id")?, Tank::from_row(&row)?))
        })
        .try_collect::<HashMap<TankId, Tank>>()
        .await
        .context("failed to retrieve the latest tank snapshots")
}

#[instrument(
    level = "debug",
    skip_all,
    fields(account_id = account_id, n_tanks = tank_ids.len()),
)]
pub async fn retrieve_latest_tank_battle_counts(
    connection: &PgPool,
    account_id: i32,
    tank_ids: &[TankId],
) -> crate::Result<HashMap<TankId, (i32, i32)>> {
    // language=SQL
    const QUERY: &str = "
        SELECT snapshot.tank_id, snapshot.battles, snapshot.wins
        FROM UNNEST($2) external_tank_id
        CROSS JOIN LATERAL (
            SELECT * FROM tank_snapshots snapshot
            WHERE
                snapshot.account_id = $1
                AND snapshot.tank_id = external_tank_id
            ORDER BY snapshot.last_battle_time DESC
            LIMIT 1
        ) snapshot
    ";

    let start_instant = Instant::now();
    let result = sqlx::query(QUERY)
        .bind(account_id)
        .bind(&tank_ids.iter().map(|tank_id| *tank_id as i32).collect_vec())
        .fetch(connection)
        .map(|row| {
            let row = row?;
            sqlx::Result::Ok((
                try_get::<i32, TankId>(&row, "tank_id")?,
                (row.try_get("battles")?, row.try_get("wins")?),
            ))
        })
        .try_collect::<HashMap<TankId, (i32, i32)>>()
        .await
        .context("failed to retrieve the latest tank battle counts");
    tracing::debug!(account_id = account_id, elapsed = %format_duration(start_instant.elapsed()));
    result
}

pub async fn replace_account(
    connection: &mut PgConnection,
    account: &BaseAccountInfo,
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

pub async fn insert_account_if_not_exists(
    connection: &PgPool,
    account_id: i32,
) -> crate::Result<DateTime> {
    // language=SQL
    const QUERY: &str = r#"
        WITH existing AS (
            INSERT INTO accounts (account_id, last_battle_time)
            VALUES ($1, TIMESTAMP WITH TIME ZONE '1970-01-01 00:00:00+00')
            ON CONFLICT (account_id) DO NOTHING
            RETURNING last_battle_time
        )
        SELECT last_battle_time FROM existing
        UNION SELECT last_battle_time FROM accounts WHERE account_id = $1;
    "#;
    sqlx::query_scalar(QUERY)
        .bind(account_id)
        .fetch_one(connection)
        .await
        .context("failed to insert the account if not exists")
}

#[allow(dead_code)]
pub async fn retrieve_account(
    connection: &PgPool,
    account_id: i32,
) -> crate::Result<Option<BaseAccountInfo>> {
    // language=SQL
    const QUERY: &str = "SELECT * FROM accounts WHERE account_id = $1";
    sqlx::query_as(QUERY)
        .bind(account_id)
        .fetch_optional(connection)
        .await
        .with_context(|| format!("failed to retrieve account #{}", account_id))
}

#[instrument(level = "debug", skip_all, fields(n_accounts = account_ids.len()))]
pub async fn retrieve_accounts(
    connection: &PgPool,
    account_ids: &[i32],
) -> crate::Result<Vec<BaseAccountInfo>> {
    // language=SQL
    const QUERY: &str = "SELECT * FROM accounts WHERE account_id IN (SELECT * FROM UNNEST($1))";
    sqlx::query_as(QUERY)
        .bind(account_ids)
        .fetch_all(connection)
        .await
        .context("failed to retrieve the accounts")
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
        )
        SELECT * FROM UNNEST($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (account_id, tank_id, last_battle_time) DO NOTHING
    ";

    // Workaround for SQLx being unable to insert multiple records at once.
    // https://github.com/launchbadge/sqlx/issues/294#issuecomment-830409187
    sqlx::query(QUERY)
        .bind(&tanks.iter().map(|tank| tank.account_id).collect_vec())
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.base.tank_id as i32)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.base.last_battle_time)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.battle_life_time.num_seconds())
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.battles)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.wins)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.survived_battles)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.win_and_survived)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.damage_dealt)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.damage_received)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.shots)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.hits)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.frags)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.all.xp)
                .collect_vec(),
        )
        .execute(&mut *connection)
        .await
        .context("failed to insert tank snapshots")?;
    Ok(())
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
            tank_id: try_convert::<i32, _>(row.try_get("tank_id")?)?,
            last_battle_time: row.try_get("last_battle_time")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for TankAchievements {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        Ok(Self {
            tank_id: try_convert::<i32, _>(row.try_get("tank_id")?)?,
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

fn try_convert<F, T>(value: F) -> Result<T, sqlx::Error>
where
    T: TryFrom<F>,
    <T as TryFrom<F>>::Error: 'static + Send + Sync + std::error::Error,
{
    value
        .try_into()
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))
}

fn try_get<'r, F, T>(row: &'r PgRow, index: &str) -> Result<T, sqlx::Error>
where
    T: TryFrom<F>,
    <T as TryFrom<F>>::Error: 'static + Send + Sync + std::error::Error,
    F: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
    try_convert::<F, T>(row.try_get(index)?)
}
