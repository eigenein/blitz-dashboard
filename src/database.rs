use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration as StdDuration, Instant};

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::{Error, Executor, FromRow, PgConnection, PgPool, Row};

pub use crate::database::mongodb::models::*;
use crate::helpers::tracing::format_elapsed;
use crate::prelude::*;
use crate::wargaming::models::{
    BasicStatistics, BasicTankStatistics, Tank, TankAchievements, TankId, TankStatistics,
};

pub mod mongodb;

/// Open and initialize the database.
#[instrument(skip_all, fields(initialize_schema), level = "debug")]
pub async fn open(uri: &str, initialize_schema: bool) -> Result<PgPool> {
    info!(uri, initialize_schema, "connecting…");
    let options = PgConnectOptions::from_str(uri)?;
    let inner = PgPoolOptions::new()
        .connect_timeout(StdDuration::from_secs(5))
        .max_connections(50)
        .connect_with(options)
        .await
        .context("failed to connect")?;

    if initialize_schema {
        info!("initializing the schema…");
        inner
            .execute(include_str!("database/script.sql"))
            .await
            .context("failed to run the script")?;
    }

    info!("ready");
    Ok(inner)
}

#[instrument(
    skip_all,
    fields(account_id = account_id, before = ?before, n_tanks = tank_ids.len()),
)]
pub async fn retrieve_latest_tank_snapshots(
    connection: &PgPool,
    account_id: i32,
    before: DateTime,
    tank_ids: &[TankId],
) -> Result<HashMap<TankId, Tank>> {
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

#[instrument(skip_all, fields(account_id = account_id))]
pub async fn retrieve_latest_tank_battle_counts(
    connection: &PgPool,
    account_id: i32,
    tank_ids: impl IntoIterator<Item = TankId>,
) -> Result<AHashMap<TankId, (i32, i32)>> {
    let tank_ids = tank_ids
        .into_iter()
        .map(|tank_id| tank_id as i32)
        .collect_vec();
    if tank_ids.is_empty() {
        return Ok(AHashMap::default());
    }

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
        .bind(&tank_ids)
        .fetch(connection)
        .map(|row| {
            let row = row?;
            sqlx::Result::Ok((
                try_get::<i32, TankId>(&row, "tank_id")?,
                (row.try_get("battles")?, row.try_get("wins")?),
            ))
        })
        .try_collect::<AHashMap<TankId, (i32, i32)>>()
        .await
        .context("failed to retrieve the latest tank battle counts");
    debug!(account_id = account_id, elapsed = format_elapsed(start_instant).as_str());
    result
}

#[instrument(skip_all, fields(n_tanks = tanks.len()))]
pub async fn insert_tank_snapshots(connection: &mut PgConnection, tanks: &[Tank]) -> Result {
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
                .map(|tank| tank.statistics.basic.tank_id as i32)
                .collect_vec(),
        )
        .bind(
            &tanks
                .iter()
                .map(|tank| tank.statistics.basic.last_battle_time)
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
            basic: BasicTankStatistics::from_row(row)?,
            battle_life_time: Duration::seconds(battle_life_time),
            all: BasicStatistics::from_row(row)?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for BasicTankStatistics {
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

impl<'r> FromRow<'r, PgRow> for BasicStatistics {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
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
