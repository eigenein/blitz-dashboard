use reqwest::StatusCode;

use crate::prelude::*;
use crate::trainer::requests::Given;
use crate::trainer::responses::RecommendResponse;
use crate::trainer::{requests, Regression};

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

    #[instrument(level = "debug", skip_all)]
    pub async fn recommend(
        &self,
        realm: wargaming::Realm,
        given: Vec<Given>,
        predict: Vec<wargaming::TankId>,
        min_prediction: f64,
    ) -> Result<RecommendResponse> {
        self.client
            .post(format!("{}/recommend", self.base_url))
            .json(&requests::RecommendRequest {
                realm,
                given,
                predict,
                min_prediction,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("the request has failed")
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn get_regression(
        &self,
        realm: wargaming::Realm,
        source_vehicle_id: wargaming::TankId,
        target_vehicle_id: wargaming::TankId,
    ) -> Result<Option<Regression>> {
        let response = self
            .client
            .get(format!(
                "{}/{}/{}/{}/regression",
                self.base_url,
                realm.to_str(),
                source_vehicle_id,
                target_vehicle_id
            ))
            .send()
            .await?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => response
                .error_for_status()?
                .json()
                .await
                .context("the request has failed"),
        }
    }
}
