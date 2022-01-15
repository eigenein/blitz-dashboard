pub mod entry;
pub mod stream;

use anyhow::Context;
use chrono::{Duration, Utc};
use redis::aio::MultiplexedConnection;
use redis::pipe;
use tracing::instrument;

use crate::battle_stream::entry::{StreamEntry, StreamEntryBuilder};
use crate::helpers::redis::{KeyValueVec, TwoTuple};

pub type Fields = KeyValueVec<String, i64>;
pub type Entry = TwoTuple<String, Fields>;
pub type StreamResponse = TwoTuple<(), Vec<Entry>>;
pub type XReadResponse = Vec<StreamResponse>;

pub const STREAM_KEY: &str = "streams::battles::v2";

const ACCOUNT_ID_KEY: &str = "a";
const TANK_ID_KEY: &str = "t";
const TIMESTAMP_KEY: &str = "ts";
const N_BATTLES_KEY: &str = "b";
const N_WINS_KEY: &str = "w";

/// Pushes the entry to the battle stream.
#[instrument(level = "debug", skip_all, fields(n_tanks = entry.tanks.len()))]
pub async fn push_entry(
    redis: &mut MultiplexedConnection,
    entry: &StreamEntry,
    stream_duration: Duration,
) -> crate::Result {
    if entry.tanks.is_empty() {
        return Ok(());
    }

    let mut pipeline = pipe();

    let mut items = vec![(ACCOUNT_ID_KEY, entry.account_id as i64)];
    for tank in &entry.tanks {
        items.extend([
            // Must start with tank ID.
            (TANK_ID_KEY, tank.tank_id as i64),
            (TIMESTAMP_KEY, tank.timestamp),
            (N_BATTLES_KEY, tank.n_battles as i64),
            (N_WINS_KEY, tank.n_wins as i64),
        ]);
    }
    pipeline.xadd(STREAM_KEY, "*", &items).ignore();

    let minimum_id = (Utc::now() - stream_duration).timestamp_millis();
    tracing::debug!(minimum_id = minimum_id, "adding the stream entries…");
    pipeline
        .cmd("XTRIM")
        .arg(STREAM_KEY)
        .arg("MINID")
        .arg("~")
        .arg(minimum_id)
        .ignore();
    pipeline
        .query_async(redis)
        .await
        .context("failed to add the entries to the stream")
}

impl TryFrom<KeyValueVec<String, i64>> for StreamEntry {
    type Error = anyhow::Error;

    fn try_from(map: KeyValueVec<String, i64>) -> crate::Result<Self> {
        let mut builder = StreamEntryBuilder::default();
        for (key, value) in map.0.into_iter() {
            match key.as_str() {
                ACCOUNT_ID_KEY => {
                    builder.account_id(value.try_into()?);
                }
                "timestamp" | TIMESTAMP_KEY => {
                    builder.timestamp(value)?;
                }
                "tank_id" | TANK_ID_KEY => {
                    builder.tank_id(value.try_into()?);
                }
                "n_battles" | N_BATTLES_KEY => {
                    builder.n_battles(value.try_into()?)?;
                }
                "n_wins" | N_WINS_KEY => {
                    builder.n_wins(value.try_into()?)?;
                }
                "is_win" => {
                    builder.n_wins(value.try_into()?)?;
                }
                _ => {}
            }
        }
        builder.build()
    }
}
