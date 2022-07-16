use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::{bson, Collection, Database};
use serde::Deserialize;

use crate::prelude::*;
use crate::wargaming;

/// Projection to select the rating chart data.
#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct RatingSnapshot {
    #[serde(rename = "lbts_millis")]
    pub date_timestamp_millis: i64,

    #[serde(rename = "close")]
    pub close_rating: wargaming::MmRating,
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
    ) -> Result<Vec<Self>> {
        let pipeline = [
            doc! {
                "$match": {
                    "rlm": realm.to_str(),
                    "aid": account_id,
                    "lbts": { "$gte": Utc::now() - Duration::days(14) },
                }
            },
            doc! { "$sort": { "lbts": -1 } },
            doc! {
                "$group": {
                    "_id": { "$subtract": [ { "$toLong": "$lbts" }, { "$mod": [ { "$toLong": "$lbts" }, 86400000 ] } ] },
                    "lbts_millis": { "$first": { "$toLong": "$lbts"} },
                    "close": { "$first": "$mm" },
                },
            },
            doc! { "$sort": { "_id": 1 } },
        ];
        let snapshots = Self::collection(from)
            .aggregate(pipeline, None)
            .await
            .context("failed to retrieve rating snapshots")?
            .try_filter_map(|document| async move { Ok(Some(bson::from_document(document)?)) })
            .try_collect::<Vec<Self>>()
            .await?;
        Ok(snapshots)
    }
}
