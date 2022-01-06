use chrono::{Duration, Utc};
use redis::aio::MultiplexedConnection;
use redis::streams::StreamReadOptions;
use redis::AsyncCommands;
use tracing::{info, instrument};

use crate::aggregator::redis::{KeyValueVec, TwoTuple, STREAM_KEY};
use crate::aggregator::stream_entry::StreamEntry;

const PAGE_SIZE: usize = 100000;

#[derive(Clone)]
pub struct Stream {
    pub entries: Vec<StreamEntry>,
    redis: MultiplexedConnection,

    /// Last read entry ID of the Redis stream.
    pointer: String,

    time_span: Duration,
}

impl Stream {
    #[instrument(skip_all, fields(time_span = %time_span))]
    pub async fn read(redis: MultiplexedConnection, time_span: Duration) -> crate::Result<Self> {
        let mut this = Self::new(redis, time_span);
        this.refresh().await?;
        Ok(this)
    }

    pub fn new(redis: MultiplexedConnection, time_span: Duration) -> Self {
        Self {
            entries: Vec::new(),
            redis,
            time_span,
            pointer: (Utc::now() - time_span).timestamp_millis().to_string(),
        }
    }

    #[tracing::instrument(level = "info", skip_all, fields(pointer = self.pointer.as_str()))]
    pub async fn refresh(&mut self) -> crate::Result {
        while {
            let n_entries = self.read_page().await?;
            tracing::info!(
                n_read = n_entries,
                n_entries = self.entries.len(),
                pointer = self.pointer.as_str(),
                "reading…",
            );
            n_entries >= PAGE_SIZE
        } {}

        self.expire();

        info!(n_entries = self.entries.len(), "refreshed");
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(pointer = self.pointer.as_str()))]
    async fn read_page(&mut self) -> crate::Result<usize> {
        type Fields = KeyValueVec<String, i64>;
        type Entry = TwoTuple<String, Fields>;
        type StreamResponse = TwoTuple<(), Vec<Entry>>;
        type XReadResponse = Vec<StreamResponse>;

        let mut response: XReadResponse = self
            .redis
            .xread_options(
                &[STREAM_KEY],
                &[&self.pointer],
                &StreamReadOptions::default().count(PAGE_SIZE),
            )
            .await?;

        match response.pop() {
            Some(TwoTuple(_, entries)) => {
                let n_entries = entries.len();
                let new_pointer = entries.last().map(|entry| &entry.0).cloned();
                for TwoTuple(_, fields) in entries.into_iter() {
                    self.entries.push(StreamEntry::try_from(fields)?);
                }
                if let Some(new_pointer) = new_pointer {
                    self.pointer = new_pointer;
                }
                Ok(n_entries)
            }
            None => Ok(0),
        }
    }

    /// Removes expired entries.
    #[tracing::instrument(level = "debug", skip_all, fields(time_span = %self.time_span))]
    fn expire(&mut self) {
        let expiry_timestamp = (Utc::now() - self.time_span).timestamp();
        self.entries
            .retain(|point| point.timestamp > expiry_timestamp);
    }
}
