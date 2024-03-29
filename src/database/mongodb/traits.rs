use std::fmt::Debug;

use futures::TryStreamExt;
use mongodb::bson::Document;
use mongodb::options::{FindOptions, UpdateModifications, UpdateOptions, WriteConcern};
use mongodb::{Collection, Database, IndexModel};
use serde::de::DeserializeOwned;
use tokio::spawn;
use tokio::time::timeout;

use crate::prelude::*;

#[async_trait]
pub trait TypedDocument: 'static + Sized + Send + Sync + DeserializeOwned + Unpin {
    const NAME: &'static str;

    #[inline]
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection(Self::NAME)
    }

    #[inline]
    async fn find_vec(
        in_: &Database,
        filter: impl Into<Option<Document>> + Send,
        options: impl Into<Option<FindOptions>> + Send,
    ) -> Result<Vec<Self>> {
        Self::collection(in_)
            .find(filter, options)
            .await
            .map_err(|error| anyhow!("failed to search in `{}`: {:#}", Self::NAME, error))?
            .try_collect()
            .await
            .map_err(|error| anyhow!("failed to collect from `{}`: {:#}", Self::NAME, error))
    }
}

#[async_trait]
pub trait Indexes: TypedDocument + Sync {
    type I: IntoIterator<Item = IndexModel> + Send;

    fn indexes() -> Self::I;

    #[instrument(skip_all, err)]
    async fn ensure_indexes(on: &Database) -> Result {
        Self::collection(on)
            .create_indexes(Self::indexes(), None)
            .await
            .with_context(|| format!("failed to create the indexes in `{}`", Self::NAME))?;
        Ok(())
    }
}

#[async_trait]
pub trait Upsert: TypedDocument {
    type Update: 'static + Into<UpdateModifications> + Debug + Send;

    fn query(&self) -> Document;

    fn update(&self) -> Result<Self::Update>;

    #[instrument(level = "debug", skip_all, fields(collection = Self::NAME))]
    async fn upsert(&self, to: &Database) -> Result {
        let query = self.query();
        let update = self.update()?;
        let options = Self::upsert_options();

        debug!(?query, ?update, "upserting…");
        let start_instant = Instant::now();
        let collection = Self::collection(to);
        let future = spawn(async move { collection.update_one(query, update, options).await });
        timeout(time::Duration::from_secs(10), future)
            .await
            .with_context(|| format!("timed out to upsert into `{}`", Self::NAME))??
            .with_context(|| format!("failed to upsert into `{}`", Self::NAME))?;

        debug!(elapsed = ?start_instant.elapsed(), "upserted");
        Ok(())
    }

    #[inline]
    fn upsert_options() -> UpdateOptions {
        let write_concern = WriteConcern::builder()
            .w_timeout(time::Duration::from_secs(5))
            .build();
        UpdateOptions::builder()
            .upsert(true)
            .write_concern(write_concern)
            .build()
    }
}
