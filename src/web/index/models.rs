use std::sync::Arc;

use anyhow::{anyhow, Context};
use async_std::sync::Mutex;
use lazy_static::lazy_static;
use lru_time_cache::LruCache;
use serde::Deserialize;
use tide::Request;

use crate::models::Account;
use crate::web::partials::SEARCH_QUERY_LENGTH;
use crate::web::state::State;

lazy_static! {
    static ref MODEL_CACHE: Arc<Mutex<LruCache<String, Arc<IndexViewModel>>>> =
        Arc::new(Mutex::new(LruCache::with_expiry_duration_and_capacity(
            std::time::Duration::from_secs(86400),
            1000
        )));
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
                let mut cache = MODEL_CACHE.lock().await;
                match cache.get(&search) {
                    Some(model) => model.clone(),
                    None => {
                        let model = Arc::new(Self {
                            accounts: Some(request.state().api.search_accounts(&search).await?),
                        });
                        cache.insert(search.clone(), model.clone());
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
