use mongodb::bson::{doc, Document};
use mongodb::options::IndexOptions;
use mongodb::{bson, IndexModel};
use serde::{Deserialize, Serialize};

use crate::database::mongodb::traits::*;
use crate::prelude::*;
use crate::wargaming;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
pub struct VehicleModel {
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "_id")]
    pub tank_id: wargaming::TankId,

    #[serde(rename = "sim")]
    pub similar: Vec<SimilarVehicle>,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
pub struct SimilarVehicle {
    #[serde_as(as = "TryFromInto<i32>")]
    #[serde(rename = "tid")]
    pub tank_id: wargaming::TankId,

    #[serde(rename = "sim")]
    pub similarity: f64,
}

impl TypedDocument for VehicleModel {
    const NAME: &'static str = "vehicle_models";
}

impl Indexes for VehicleModel {
    type I = [IndexModel; 1];

    fn indexes() -> Self::I {
        [IndexModel::builder()
            .keys(doc! { "_id": 1, "sim.tid": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build()]
    }
}

impl Upsert for VehicleModel {
    type Update = Document;

    fn query(&self) -> Document {
        doc! { "_id": self.tank_id }
    }

    fn update(&self) -> Result<Self::Update> {
        Ok(doc! { "$set": { "sim": bson::to_bson(&self.similar)? } })
    }
}
