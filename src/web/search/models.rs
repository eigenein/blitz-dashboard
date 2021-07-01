use anyhow::{anyhow, Context};
use serde::Deserialize;
use tide::Request;

use crate::models::AccountInfo;
use crate::web::partials::SEARCH_QUERY_LENGTH;
use crate::web::state::State;
use itertools::Itertools;

pub struct ViewModel {
    pub query: String,
    pub accounts: Vec<AccountInfo>,
}

impl ViewModel {
    pub async fn new(request: &Request<State>) -> crate::Result<Self> {
        let query: Query = request
            .query()
            .map_err(|error| anyhow!(error))
            .context("failed to parse the query")?;
        query.validate()?;

        let state = request.state();
        let accounts = state
            .api
            .get_account_info(
                state
                    .search_accounts(&query.query)
                    .await?
                    .iter()
                    .map(|account| account.id),
            )
            .await?
            .drain()
            .filter_map(|(_, info)| info)
            .sorted_unstable_by(|left, right| left.nickname.cmp(&right.nickname))
            .collect();
        Ok(Self {
            accounts,
            query: query.query,
        })
    }
}

#[derive(Deserialize)]
struct Query {
    query: String,
}

impl Query {
    fn validate(&self) -> crate::Result {
        if SEARCH_QUERY_LENGTH.contains(&self.query.len()) {
            Ok(())
        } else {
            Err(anyhow!("invalid search query length"))
        }
    }
}
