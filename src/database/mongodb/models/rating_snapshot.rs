use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::{bson, Collection, Database};
use serde::Deserialize;

use crate::math::traits::MMRating;
use crate::prelude::*;
use crate::wargaming;

/// Projection to select only timestamps and ratings from account snapshots.
#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct RatingSnapshot {
    #[serde(rename = "lbts")]
    #[serde_as(as = "bson::DateTime")]
    pub last_battle_time: DateTime,

    #[serde(rename = "mm")]
    pub mm_rating: f64,
}

impl MMRating for RatingSnapshot {
    fn mm_rating(&self) -> f64 {
        self.mm_rating
    }
}

impl RatingSnapshot {
    fn collection(in_: &Database) -> Collection<Self> {
        in_.collection("account_snapshots")
    }
}

impl RatingSnapshot {
    #[instrument(level = "debug", skip_all, fields(realm = ?realm, account_id = account_id))]
    pub async fn retrieve_latest(
        from: &Database,
        realm: wargaming::Realm,
        account_id: wargaming::AccountId,
        count: i64,
    ) -> Result<Vec<Self>> {
        let filter = doc! {
            "rlm": realm.to_str(),
            "aid": account_id,
        };
        let options = FindOptions::builder()
            .sort(doc! {"lbts": -1})
            .projection(doc! {"_id": 0, "lbts": 1, "mm": 1})
            .limit(count)
            .build();
        let snapshots = Self::collection(from)
            .find(filter, options)
            .await?
            .try_collect()
            .await?;
        Ok(snapshots)
    }
}
