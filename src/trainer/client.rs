use reqwest::StatusCode;

use crate::prelude::*;
use crate::trainer::{requests, responses};

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let this = Self {
            client: reqwest::ClientBuilder::new()
                .timeout(time::Duration::from_secs(1))
                .connect_timeout(time::Duration::from_secs(1))
                .build()?,
            base_url: base_url.into(),
        };
        Ok(this)
    }

    pub async fn get_vehicle(
        &self,
        to_tank_id: wargaming::TankId,
    ) -> Result<Option<responses::VehicleResponse>> {
        let response = self
            .client
            .get(format!("{}/vehicles/{}", self.base_url, to_tank_id))
            .send()
            .await?;
        if response.status() != StatusCode::NOT_FOUND {
            Ok(Some(response.error_for_status()?.json().await?))
        } else {
            Ok(None)
        }
    }

    pub async fn recommend(
        &self,
        given: Vec<(wargaming::TankId, f64)>,
        predict: Vec<wargaming::TankId>,
    ) -> Result<Vec<(wargaming::TankId, f64)>> {
        let response = self
            .client
            .post(format!("{}/recommend", self.base_url))
            .json(&requests::RecommendRequest { given, predict })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response)
    }
}
