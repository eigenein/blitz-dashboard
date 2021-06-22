use std::sync::Arc;

use anyhow::{anyhow, Context};
use serde::Deserialize;
use tide::Request;

use crate::models::Account;
use crate::web::components::SEARCH_QUERY_LENGTH;
use crate::web::state::State;

pub struct IndexViewModel {
    pub accounts: Option<Arc<Vec<Account>>>,
}

impl IndexViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<Self> {
        let search = request
            .query::<Query>()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?
            .search;
        log::debug!("Index: {:?}", search);
        if let Some(search) = search {
            if SEARCH_QUERY_LENGTH.contains(&search.len()) {
                return Ok(Self {
                    accounts: Some(request.state().search_accounts(search).await?),
                });
            }
        }
        Ok(Self { accounts: None })
    }
}

#[derive(Deserialize)]
struct Query {
    #[serde(default = "Option::default")]
    search: Option<String>,
}
