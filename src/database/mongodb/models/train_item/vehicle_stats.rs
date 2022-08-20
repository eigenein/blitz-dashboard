use futures::TryStreamExt;
use mongodb::bson::{doc, from_document};
use serde::Deserialize;

use crate::database::mongodb::traits::TypedDocument;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct VehicleStats {
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "_id")]
    pub tank_id: wargaming::TankId,

    pub victory_ratio: f64,
}

impl TypedDocument for VehicleStats {
    const NAME: &'static str = "train";
}

impl VehicleStats {
    #[instrument(skip_all)]
    pub async fn retrieve_all(
        from: &mongodb::Database,
        realm: wargaming::Realm,
        after: DateTime,
    ) -> Result<AHashMap<wargaming::TankId, Self>> {
        let pipeline = [
            doc! { "$match": { "rlm": realm.to_str(), "lbts": { "$gte": after } } },
            doc! { "$group": { "_id": "$tid", "n_battles": { "$sum": "$nb" }, "n_wins": { "$sum": "$nw" } } },
            doc! { "$match": { "n_battles": { "$ne": 0 } } },
            doc! { "$project": { "victory_ratio": { "$divide": [ "$n_wins", "$n_battles" ] } } },
        ];
        Self::collection(from)
            .aggregate(pipeline, None)
            .await?
            .try_filter_map(|document| async move {
                trace!(?document);
                let stats = from_document::<Self>(document)?;
                Ok(Some((stats.tank_id, stats)))
            })
            .try_collect()
            .await
            .context("failed to collect the vehicle stats")
    }
}
