use crate::prelude::*;
use crate::trainer::requests;
use crate::trainer::requests::Given;

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

    pub async fn recommend(
        &self,
        realm: wargaming::Realm,
        given: Vec<Given>,
        predict: Vec<wargaming::TankId>,
    ) -> Result<Vec<(wargaming::TankId, f64)>> {
        let response = self
            .client
            .post(format!("{}/recommend", self.base_url))
            .json(&requests::RecommendRequest {
                realm,
                given,
                predict,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response)
    }
}
