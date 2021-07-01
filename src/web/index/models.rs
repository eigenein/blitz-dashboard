use std::sync::Arc;

use anyhow::{anyhow, Context};
use serde::Deserialize;
use tide::Request;

use crate::models::Account;
use crate::web::partials::SEARCH_QUERY_LENGTH;
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
        let model = match search {
            Some(search) if SEARCH_QUERY_LENGTH.contains(&search.len()) => {
                log::debug!("Searching accounts by {}…", &search);
                Self {
                    accounts: Some(request.state().search_accounts(search).await?),
                }
            }
            _ => Self { accounts: None },
        };
        Ok(model)
    }
}

#[derive(Deserialize)]
struct Query {
    #[serde(default = "Option::default")]
    search: Option<String>,
}
