use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Context};
use lazy_static::lazy_static;
use moka::future::{Cache, CacheBuilder};
use serde::Deserialize;
use tide::Request;

use crate::models::Account;
use crate::web::partials::SEARCH_QUERY_LENGTH;
use crate::web::state::State;

lazy_static! {
    static ref MODEL_CACHE: Cache<String, Arc<IndexViewModel>> = CacheBuilder::new(1_000)
        .time_to_live(StdDuration::from_secs(86400))
        .build();
    static ref DEFAULT_MODEL: Arc<IndexViewModel> = Arc::new(IndexViewModel { accounts: None });
}

pub struct IndexViewModel {
    pub accounts: Option<Vec<Account>>,
}

impl IndexViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<Arc<Self>> {
        let search = request
            .query::<Query>()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?
            .search;
        let model = match search {
            Some(search) if SEARCH_QUERY_LENGTH.contains(&search.len()) => {
                log::debug!("Searching accounts by {}…", &search);
                match MODEL_CACHE.get(&search) {
                    Some(model) => model,
                    None => {
                        let model = Arc::new(Self {
                            accounts: Some(request.state().api.search_accounts(&search).await?),
                        });
                        MODEL_CACHE.insert(search.clone(), model.clone()).await;
                        model
                    }
                }
            }
            _ => DEFAULT_MODEL.clone(),
        };
        Ok(model)
    }
}

#[derive(Deserialize)]
struct Query {
    #[serde(default = "Option::default")]
    search: Option<String>,
}
