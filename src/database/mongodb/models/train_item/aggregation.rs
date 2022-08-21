use bpci::{Interval, NSuccessesSample, WilsonScore};
use futures::{Stream, TryStreamExt};
use mongodb::bson::{doc, from_document};
use serde::Deserialize;

use crate::database::mongodb::traits::TypedDocument;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Deserialize, Debug)]
pub struct TrainAggregation {
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "aid")]
    pub account_id: wargaming::AccountId,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "tid")]
    pub tank_id: wargaming::TankId,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "nb")]
    pub n_battles: u32,

    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "nw")]
    pub n_wins: u32,
}

impl TypedDocument for TrainAggregation {
    const NAME: &'static str = "train";
}

impl TrainAggregation {
    pub fn victory_ratio(&self, z_level: f64) -> Result<f64> {
        Ok(NSuccessesSample::new(self.n_battles, self.n_wins)?
            .wilson_score_with_cc(z_level)
            .mean())
    }

    #[instrument(skip_all)]
    pub async fn aggregate_by_vehicles(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        since: DateTime,
    ) -> Result<Vec<Self>> {
        let pipeline = [
            doc! { "$match": { "rlm": realm.to_str(), "lbts": { "$gte": since } } },
            doc! {
                "$group": {
                    "_id": "$tid",
                    "nb": { "$sum": "$nb" },
                    "nw": { "$sum": { "$ifNull": [ "$nw", 0 ] } }
                }
            },
            doc! { "$match": { "$expr": { "$ne": [ "$nb", 0 ] } } },
            doc! { "$set": { "aid": 0, "tid": "$_id" } },
            doc! { "$unset": "_id" },
        ];
        Self::collection(from)
            .aggregate(pipeline, None)
            .await?
            .try_filter_map(|document| async move {
                trace!(?document);
                Ok(Some(from_document::<Self>(document)?))
            })
            .try_collect()
            .await
            .context("failed to collect the vehicle stats")
    }

    #[instrument(skip_all, fields(since = ?since))]
    pub async fn aggregate_by_account_tanks(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        since: DateTime,
    ) -> Result<impl Stream<Item = Result<Self>>> {
        let pipeline = [
            doc! { "$match": { "rlm": realm.to_str(), "lbts": { "$gte": since } } },
            doc! {
                "$group": {
                    "_id": { "aid": "$aid", "tid": "$tid" },
                    "nb": { "$sum": "$nb" },
                    "nw": { "$sum": { "$ifNull": [ "$nw", 0 ] } }
                }
            },
            doc! { "$match": { "$expr": { "$ne": [ "$nb", 0 ] } } },
            doc! { "$set": { "aid": "$_id.aid", "tid": "$_id.tid" } },
            doc! { "$unset": "_id" },
        ];
        let stream = Self::collection(from)
            .aggregate(pipeline, None)
            .await?
            .map_err(Error::from)
            .try_filter_map(|document| async move {
                trace!(?document);
                Ok(Some(from_document::<Self>(document)?))
            });
        Ok(stream)
    }
}
