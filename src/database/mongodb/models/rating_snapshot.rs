use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, IndexOptions};
use mongodb::{bson, Collection, Database, IndexModel};
use serde::Deserialize;
use serde_with::TryFromInto;
use tokio::spawn;
use tokio::time::timeout;

use crate::database::mongodb::options::upsert_options;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct RatingSnapshot {
    #[serde(rename = "rlm")]
    pub realm: wargaming::Realm,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde(default, rename = "szn")]
    pub season: u16,

    /// Even though this a `DateTime`, actually the time is always 00:00:00.
    /// We're only interested in a date.
    #[serde(rename = "dt")]
    #[serde_as(as = "bson::DateTime")]
    pub date: DateTime,

    /// The last seen rating during the day.
    #[serde(default, rename = "cl")]
    pub close_rating: wargaming::MmRating,
}

/// Implements ways to instantiate a structure.
impl RatingSnapshot {
    pub fn new(realm: wargaming::Realm, account_info: &wargaming::AccountInfo) -> Option<Self> {
        let has_rating = account_info.stats.rating.current_season != 0
            // This is bad, but there's no telling other than that.
            && account_info.stats.rating.mm_rating.0 != 0.0;
        has_rating.then(|| Self {
            realm,
            account_id: account_info.id,
            season: account_info.stats.rating.current_season,
            date: account_info.last_battle_time.date().and_hms(0, 0, 0),
            close_rating: account_info.stats.rating.mm_rating,
        })
    }
}

/// Implements the common database-related functions.
impl RatingSnapshot {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("rating_snapshots")
    }

    #[instrument(skip_all, err)]
    pub async fn ensure_indexes(on: &Database) -> Result {
        let indexes = [IndexModel::builder()
            .keys(doc! { "rlm": 1, "aid": 1, "szn": -1, "dt": -1 })
            .options(IndexOptions::builder().unique(true).build())
            .build()];
        Self::collection(on)
            .create_indexes(indexes, None)
            .await
            .context("failed to create the indexes on rating snapshots")?;
        Ok(())
    }
}

/// Implements logic-related queries.
impl RatingSnapshot {
    #[instrument(
        skip_all,
        fields(account_id = self.account_id),
        err,
    )]
    pub async fn upsert(&self, to: &Database) -> Result {
        let query = doc! {
            "rlm": self.realm.to_str(),
            "aid": self.account_id,
            "szn": self.season as i32,
            "dt": self.date,
        };
        let update = doc! { "$set": { "cl": self.close_rating.0 } };
        let options = upsert_options();

        debug!("upserting…");
        let start_instant = Instant::now();
        let collection = Self::collection(to);
        let future = spawn(async move { collection.update_one(query, update, options).await });
        timeout(StdDuration::from_secs(10), future)
            .await
            .context("timed out to insert the rating snapshot")??
            .context("failed to upsert the rating snapshot")?;

        debug!(elapsed = ?start_instant.elapsed(), "upserted");
        Ok(())
    }

    #[instrument(
        skip_all,
        level = "debug",
        fields(realm = ?realm, account_id = account_id, season = season),
    )]
    pub async fn retrieve_season(
        from: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        season: u16,
    ) -> Result<Vec<Self>> {
        let filter = doc! {
            "rlm": realm.to_str(),
            "aid": account_id,
            "szn": season as i32
        };
        let options = FindOptions::builder().sort(doc! { "dt": -1 }).build();
        let snapshots = Self::collection(from)
            .find(filter, options)
            .await
            .context("failed to query the ratings")?
            .try_collect()
            .await
            .context("failed to collect the ratings")?;
        Ok(snapshots)
    }
}
