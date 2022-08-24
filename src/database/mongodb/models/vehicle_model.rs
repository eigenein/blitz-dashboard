use mongodb::bson;
use mongodb::bson::{doc, Document};
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
    pub similarities: Vec<(wargaming::TankId, f64)>,
}

impl TypedDocument for VehicleModel {
    const NAME: &'static str = "vehicle_models";
}

impl Upsert for VehicleModel {
    type Update = Document;

    fn query(&self) -> Document {
        doc! { "_id": self.tank_id }
    }

    fn update(&self) -> Result<Self::Update> {
        Ok(doc! { "$set": { "sim": bson::to_bson(&self.similarities)? } })
    }
}
