use std::any::type_name;
use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;
use lru_time_cache::LruCache;
use mongodb::bson::doc;
use serde::Deserialize;
use tide::Request;

use crate::cached::Cached;
use crate::wargaming::models::Account;
use crate::web::components::SEARCH_QUERY_LENGTH;
use crate::web::State;

lazy_static! {
    /// Caches search results for a day.
    static ref MODEL_CACHE: Cached<String, IndexViewModel> = Cached::new(
        LruCache::with_expiry_duration_and_capacity(Duration::from_secs(86400), 1000)
    );
}

/// User search query.
#[derive(Deserialize)]
struct IndexQueryString {
    #[serde(default = "Option::default")]
    search: Option<String>,
}

pub struct IndexViewModel {
    pub accounts: Option<Vec<Account>>,
}

impl IndexViewModel {
    pub async fn new(request: Request<State>) -> crate::Result<Arc<Self>> {
        let query: IndexQueryString = request.query().map_err(surf::Error::into_inner)?;
        log::debug!("{} {:?}…", type_name::<Self>(), query.search);
        if let Some(query) = query.search {
            if SEARCH_QUERY_LENGTH.contains(&query.len()) {
                return MODEL_CACHE
                    .get(&query, || async {
                        Ok(Self {
                            accounts: Some(request.state().api.search_accounts(&query).await?),
                        })
                    })
                    .await;
            }
        }
        Ok(Arc::new(Self { accounts: None }))
    }
}
